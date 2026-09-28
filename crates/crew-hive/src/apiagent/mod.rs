//! Native API agent: calls a [`Provider`] in an async future, emitting
//! telemetry events as it goes. The default headless scale worker.

#[cfg(test)]
#[path = "preamble_tests.rs"]
mod preamble_tests;
#[cfg(test)]
#[path = "samecall_tests.rs"]
mod samecall_tests;
#[cfg(test)]
#[path = "shrinkold_tests.rs"]
mod shrinkold_tests;
#[cfg(test)]
#[path = "stop_tests.rs"]
mod stop_tests;
#[cfg(test)]
mod tests;

mod chunks;
mod context;
mod cost;
mod failure;
mod lastword;
mod native;
mod note;
mod pending;
mod textround;
mod toolloop;

pub(crate) use context::build_prompt;
pub(crate) use cost::billed;
pub(crate) use failure::reason;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::agent::{Agent, AgentContext, Attempt};
use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::graph::AgentKind;
use crate::provider::{CompletionRequest, Provider};
use crate::tools::{self, ToolCatalog, Tools};

pub struct ApiAgent {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
    /// The tool surface, or `None` for a text-only agent. `None` must behave
    /// EXACTLY as this agent did before tools existed — same prompt bytes,
    /// same events, same single provider call — because that is still the
    /// configuration every keyless and mock run uses.
    tools: Option<Arc<dyn Tools>>,
    /// Standing context every task's system prompt ends with — the host's
    /// project card. A worker sees only its own task text, which the planner
    /// rewrote ("Define crew-term"), so without this it cannot know the
    /// thing it was asked about is a directory beside it.
    preamble: Option<String>,
}

impl ApiAgent {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
            tools: None,
            preamble: None,
        }
    }

    /// End every task's system prompt with `preamble`.
    pub fn with_preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }

    /// Let this agent call tools between provider calls.
    pub fn with_tools(mut self, tools: Arc<dyn Tools>) -> Self {
        self.tools = Some(tools);
        self
    }
}

impl Agent for ApiAgent {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let attempt = self.attempt(ctx);
        Box::pin(async move { attempt.await.result })
    }

    /// One task: call the provider, and while the reply asks for a tool, run
    /// it and call the provider again with the result. A provider error ends
    /// it, said in the output and marked passing or not (`failure`).
    ///
    /// With no tool surface attached this is exactly one provider call and the
    /// same three events it always published — the loop's first pass IS the
    /// old body, and `split_tool_calls` is never even reached.
    fn attempt(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = Attempt> + Send>> {
        let provider = Arc::clone(&self.provider);
        let max_tokens = self.max_tokens;
        let model = self.model.clone();
        let tools = self.tools.clone();
        let preamble = self.preamble.clone();
        Box::pin(async move {
            let task_id = ctx.task.id;
            let agent_id = ctx.agent.clone();
            // Honour the per-task model tier the planner assigned, not a fixed
            // factory tier — this is what lets a plan mix cheap and capable
            // models. The BILL follows the model id that actually answered
            // (`cost::billed`); the tier is only its fallback.
            let tier = ctx.task.model;
            let system = match &ctx.task.agent {
                AgentKind::Api { system } => system.clone(),
                AgentKind::Pty { .. } => None,
            };
            let system = with_preamble(system, preamble.as_deref());
            let model_id = model.unwrap_or_else(|| tier.model_id().to_owned());
            // NATIVE OR TEXT, decided once. Native needs BOTH halves — schemas
            // from the tool surface and tool support from the provider — and
            // when either is missing the `@tool` convention still works, which
            // is why it stays rather than being replaced.
            if let Some(runner) = tools.clone() {
                let specs = runner.specs_for(&ctx.task.prompt);
                if !specs.is_empty() && provider.supports_tools() {
                    let sink = chunks::ChunkSink::new(ctx.bus.clone(), agent_id.clone());
                    // No tools hint in the prompt: the tools are on the wire,
                    // and advertising the text convention beside them invites
                    // a model to use both. What was left OFF the wire is the
                    // one thing said in prose (`Tools::note_for`, `note.rs`).
                    let (system, prompt) = note::noted(
                        system,
                        build_prompt(&ctx.task.prompt, &ctx.deps),
                        runner.note_for(&ctx.task.prompt),
                    );
                    return native::run(
                        ctx,
                        provider,
                        runner,
                        ToolCatalog::build(&specs),
                        model_id,
                        system,
                        prompt,
                        max_tokens,
                        sink,
                    )
                    .await;
                }
            }
            // The tools section is part of the BASE prompt, not just the first
            // request: every follow-up re-states it, or an agent that used one
            // tool would find the syntax for the second one gone.
            let body = build_prompt(&ctx.task.prompt, &ctx.deps);
            let base = tools::augment(
                &body,
                &tools
                    .as_ref()
                    .map(|t| t.hint_for(&ctx.task.prompt))
                    .unwrap_or_default(),
            );
            // Fragments publish as they arrive. NOTE that with tool rounds the
            // deltas now carry MORE than the final `OutputChunk`: every round's
            // thinking streams, while the chunk is the ANSWER. That is the
            // right split — the intervening rounds are published as their own
            // ToolCall/ToolResult events, so nothing is lost, and a transcript
            // built from chunks stays the answer rather than the working.
            let sink = chunks::ChunkSink::new(ctx.bus.clone(), agent_id.clone());

            let mut prompt = base.clone();
            let mut exchanges = tools::exchanges::Exchanges::default();
            let mut round: u32 = 0;
            // Reads this task has made. A repeat is answered from the result
            // already in `exchanges`, while the follow-up still carries it
            // whole (`Exchanges::repeat`); once shortened, it runs again.
            let mut seen = tools::seen::Seen::default();
            loop {
                // A stop pressed while the last round's tools ran: no next
                // call. The tools already run are not undone (`stop`).
                if ctx.stopped() {
                    return Attempt::stopped(task_id);
                }
                let req = CompletionRequest {
                    model: model_id.clone(),
                    system: system.clone(),
                    prompt,
                    max_tokens,
                    ..Default::default()
                };
                // Raced against the stop: a call nobody wants any more is
                // dropped, which aborts the request, rather than waited out.
                let call = provider.complete_streaming(req, Arc::clone(&sink.on_chunk));
                let completion = match ctx.unless_stopped(call).await {
                    None => return Attempt::stopped(task_id),
                    Some(Ok(c)) => c,
                    Some(Err(err)) => return failure::failed(&ctx, &err),
                };
                sink.settle(&completion);
                // Billed per round, as it happens: a run that spends four
                // model calls on tools must not look like one call's worth of
                // tokens to the budget governor watching this bus.
                ctx.bus.publish(HiveEvent::TokenDelta {
                    agent: agent_id.clone(),
                    input: completion.input_tokens,
                    output: completion.output_tokens,
                });
                ctx.bus.publish(HiveEvent::CostDelta {
                    agent: agent_id.clone(),
                    micros_usd: cost::billed(&model_id, tier, &completion),
                });

                let asked = tools
                    .as_ref()
                    .and_then(|_| tools::split_tool_calls(&completion.text));
                let (Some(runner), Some((said, calls))) = (tools.as_ref(), asked) else {
                    // No tool asked for: this reply is the answer.
                    ctx.bus.publish(HiveEvent::OutputChunk {
                        agent: agent_id,
                        text: completion.text.clone(),
                    });
                    return TaskResult {
                        task: task_id,
                        output: completion.text,
                        success: true,
                    }
                    .into();
                };
                // Every call draws its own round; the ones granted run as one
                // batch and each becomes its own exchange (`textround`).
                let (granted, rounds_left) = textround::draw(&ctx, &mut round, calls.len());
                let (run, refused) = calls.split_at(granted);
                if !run.is_empty() {
                    textround::run(&ctx, runner, &mut seen, &mut exchanges, &said, run).await;
                }
                if !refused.is_empty() {
                    // Asked for more with the budget gone. Say so in the
                    // output rather than returning an unrun directive that
                    // reads like a call which happened.
                    let total = ctx.budget.total();
                    for call in refused {
                        ctx.bus.publish(HiveEvent::ToolResult {
                            agent: agent_id.clone(),
                            label: call.label(),
                            ok: false,
                            text: format!("not run — tool budget spent ({total} calls this run)"),
                            ms: 0,
                        });
                    }
                    // What it gathered goes back once more, tools hint left
                    // out, for an answer (`lastword`); failing that, what it
                    // had written beside the ask is all there is. The message
                    // is already in the log when any call of the reply ran.
                    let said = if run.is_empty() { said.as_str() } else { "" };
                    let asked = refused.iter().map(|c| (c.label(), c.args.clone()));
                    let last = lastword::prompt(
                        &body,
                        &exchanges.render(),
                        &lastword::refused(said, asked),
                    );
                    let answer =
                        lastword::ask(&ctx, &provider, &model_id, system, last, max_tokens, &sink)
                            .await;
                    if ctx.stopped() {
                        return Attempt::stopped(task_id);
                    }
                    let text = match answer {
                        Some(answer) => toolloop::with_budget_note(&answer, total),
                        None => toolloop::budget_spent(&completion.text, total),
                    };
                    ctx.bus.publish(HiveEvent::OutputChunk {
                        agent: agent_id,
                        text: text.clone(),
                    });
                    return TaskResult {
                        task: task_id,
                        output: text,
                        success: true,
                    }
                    .into();
                }
                prompt = toolloop::follow_up(&base, &exchanges, rounds_left);
            }
        })
    }
}

use crate::agent::AgentFactory;

/// Agent factory making native [`ApiAgent`]s that share one provider. Each
/// agent reads its model tier from its task at run time (see [`ApiAgent::attempt`]),
/// so the factory only needs the provider and the per-task output token cap.
pub struct ApiFactory {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
    /// Shared by every agent the factory makes, so one MCP host and ONE
    /// approval gate serve the whole swarm. Handing each agent its own would
    /// mean a person approving the same irreversible tool once per agent.
    tools: Option<Arc<dyn Tools>>,
    preamble: Option<String>,
}

impl ApiFactory {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
            tools: None,
            preamble: None,
        }
    }

    /// End every agent's system prompt with `preamble` (see [`ApiAgent::with_preamble`]).
    pub fn with_preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }

    /// Give every agent this factory makes the same tool surface.
    pub fn with_tools(mut self, tools: Arc<dyn Tools>) -> Self {
        self.tools = Some(tools);
        self
    }
}

/// `system` with `preamble` after it — or the preamble alone, or neither.
fn with_preamble(system: Option<String>, preamble: Option<&str>) -> Option<String> {
    match (system, preamble) {
        (s, None) => s,
        (None, Some(p)) => Some(p.to_string()),
        (Some(s), Some(p)) => Some(format!("{s}\n\n{p}")),
    }
}

impl AgentFactory for ApiFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        let mut agent = ApiAgent::new(Arc::clone(&self.provider), self.max_tokens);
        if let Some(m) = &self.model {
            agent = agent.with_model(m.clone());
        }
        if let Some(t) = &self.tools {
            agent = agent.with_tools(Arc::clone(t));
        }
        if let Some(p) = &self.preamble {
            agent = agent.with_preamble(p.clone());
        }
        Box::new(agent)
    }
}
