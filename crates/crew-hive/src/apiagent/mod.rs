//! Native API agent: calls a [`Provider`] in an async future, emitting
//! telemetry events as it goes. The default headless scale worker.

#[cfg(test)]
mod tests;

mod context;
mod native;
mod toolloop;

pub(crate) use context::build_prompt;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::agent::{Agent, AgentContext};
use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::graph::{AgentKind, ModelTier};
use crate::provider::{CompletionRequest, Provider};
use crate::tools::{self, ToolCatalog, Tools, MAX_TOOL_ROUNDS};

// ---------------------------------------------------------------------------
// Cost table — micros-USD per token (input / output)
// ---------------------------------------------------------------------------

/// Approximate Anthropic pricing in micros-USD per token.
/// Cheap ≈ $1/$5 per 1M; Standard ≈ $3/$15; Capable ≈ $15/$75.
fn cost_micros(tier: ModelTier, input: u32, output: u32) -> u64 {
    let (in_rate, out_rate): (u64, u64) = match tier {
        ModelTier::Cheap => (1, 5),
        ModelTier::Standard => (3, 15),
        ModelTier::Capable => (15, 75),
    };
    in_rate * u64::from(input) + out_rate * u64::from(output)
}

// ---------------------------------------------------------------------------
// ApiAgent
// ---------------------------------------------------------------------------

pub struct ApiAgent {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
    /// The tool surface, or `None` for a text-only agent. `None` must behave
    /// EXACTLY as this agent did before tools existed — same prompt bytes,
    /// same events, same single provider call — because that is still the
    /// configuration every keyless and mock run uses.
    tools: Option<Arc<dyn Tools>>,
}

impl ApiAgent {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
            tools: None,
        }
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
    /// One task: call the provider, and while the reply asks for a tool, run
    /// it and call the provider again with the result.
    ///
    /// With no tool surface attached this is exactly one provider call and the
    /// same three events it always published — the loop's first pass IS the
    /// old body, and `parse_tool_call` is never even reached.
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let provider = Arc::clone(&self.provider);
        let max_tokens = self.max_tokens;
        let model = self.model.clone();
        let tools = self.tools.clone();
        Box::pin(async move {
            let task_id = ctx.task.id;
            let agent_id = ctx.agent.clone();
            // Honour the per-task model tier the planner assigned, not a fixed
            // factory tier — this is what lets a plan mix cheap and capable
            // models, and bills each task at its own rate.
            let tier = ctx.task.model;
            let system = match &ctx.task.agent {
                AgentKind::Api { system } => system.clone(),
                AgentKind::Pty { .. } => None,
            };
            let model_id = model.unwrap_or_else(|| tier.model_id().to_owned());
            // NATIVE OR TEXT, decided once. Native needs BOTH halves — schemas
            // from the tool surface and tool support from the provider — and
            // when either is missing the `@tool` convention still works, which
            // is why it stays rather than being replaced.
            if let Some(runner) = tools.clone() {
                let specs = runner.specs();
                if !specs.is_empty() && provider.supports_tools() {
                    let delta_bus = ctx.bus.clone();
                    let delta_agent = agent_id.clone();
                    let on_chunk: crate::provider::ChunkFn = Arc::new(move |s: &str| {
                        delta_bus.publish(HiveEvent::OutputDelta {
                            agent: delta_agent.clone(),
                            text: s.to_string(),
                        });
                    });
                    // No tools hint in the prompt: the tools are on the wire,
                    // and advertising the text convention beside them invites
                    // a model to use both.
                    let prompt = build_prompt(&ctx.task.prompt, &ctx.deps);
                    return native::run(
                        ctx,
                        provider,
                        runner,
                        ToolCatalog::build(&specs),
                        model_id,
                        system,
                        prompt,
                        max_tokens,
                        on_chunk,
                    )
                    .await;
                }
            }
            // The tools section is part of the BASE prompt, not just the first
            // request: every follow-up re-states it, or an agent that used one
            // tool would find the syntax for the second one gone.
            let base = tools::augment(
                &build_prompt(&ctx.task.prompt, &ctx.deps),
                &tools.as_ref().map(|t| t.hint()).unwrap_or_default(),
            );
            // Fragments publish as they arrive. NOTE that with tool rounds the
            // deltas now carry MORE than the final `OutputChunk`: every round's
            // thinking streams, while the chunk is the ANSWER. That is the
            // right split — the intervening rounds are published as their own
            // ToolCall/ToolResult events, so nothing is lost, and a transcript
            // built from chunks stays the answer rather than the working.
            let delta_bus = ctx.bus.clone();
            let delta_agent = agent_id.clone();
            let on_chunk: crate::provider::ChunkFn = Arc::new(move |s: &str| {
                delta_bus.publish(HiveEvent::OutputDelta {
                    agent: delta_agent.clone(),
                    text: s.to_string(),
                });
            });

            let mut prompt = base.clone();
            let mut exchanges: Vec<String> = Vec::new();
            let mut round: u32 = 0;
            loop {
                let req = CompletionRequest {
                    model: model_id.clone(),
                    system: system.clone(),
                    prompt,
                    max_tokens,
                    ..Default::default()
                };
                let completion = match provider
                    .complete_streaming(req, Arc::clone(&on_chunk))
                    .await
                {
                    Ok(c) => c,
                    Err(err) => {
                        ctx.bus.publish(HiveEvent::Failed {
                            agent: agent_id,
                            error: err.to_string(),
                        });
                        return TaskResult {
                            task: task_id,
                            output: String::new(),
                            success: false,
                        };
                    }
                };
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
                    micros_usd: cost_micros(
                        tier,
                        completion.input_tokens,
                        completion.output_tokens,
                    ),
                });

                let call = tools
                    .as_ref()
                    .and_then(|_| tools::parse_tool_call(&completion.text));
                let (Some(runner), Some(call)) = (tools.as_ref(), call) else {
                    // No tool asked for: this reply is the answer.
                    ctx.bus.publish(HiveEvent::OutputChunk {
                        agent: agent_id,
                        text: completion.text.clone(),
                    });
                    return TaskResult {
                        task: task_id,
                        output: completion.text,
                        success: true,
                    };
                };
                if round >= MAX_TOOL_ROUNDS {
                    // Asked for one more with the budget gone. Say so in the
                    // output rather than returning an unrun directive that
                    // reads like a call which happened.
                    let text = toolloop::budget_spent(&completion.text, MAX_TOOL_ROUNDS);
                    ctx.bus.publish(HiveEvent::ToolResult {
                        agent: agent_id.clone(),
                        label: call.label(),
                        ok: false,
                        text: format!("not run — tool budget spent ({MAX_TOOL_ROUNDS} calls)"),
                    });
                    ctx.bus.publish(HiveEvent::OutputChunk {
                        agent: agent_id,
                        text: text.clone(),
                    });
                    return TaskResult {
                        task: task_id,
                        output: text,
                        success: true,
                    };
                }

                let label = call.label();
                ctx.bus.publish(HiveEvent::ToolCall {
                    agent: agent_id.clone(),
                    label: label.clone(),
                    args: call.args.clone(),
                });
                // OFF THE RUNTIME THREAD. `Tools::call` is blocking — an MCP
                // round trip, or a shell command with a two-minute deadline —
                // and the scheduler runs its agents on ONE current-thread
                // runtime alongside the bus drain. Awaiting it inline would
                // freeze every other agent in the swarm and stop events
                // reaching the pane, so the whole run would look hung for as
                // long as one tool took.
                let runner = Arc::clone(runner);
                let (server, tool, args) =
                    (call.server.clone(), call.tool.clone(), call.args.clone());
                let outcome =
                    tokio::task::spawn_blocking(move || runner.call(&server, &tool, &args))
                        .await
                        .unwrap_or_else(|e| Err(format!("tool task failed: {e}")));
                let (ok, text) = match outcome {
                    Ok(t) if t.trim().is_empty() => (true, "(empty result)".to_string()),
                    Ok(t) => (true, t),
                    // A refused or failed tool is shown to the agent, not
                    // raised as a task failure: "that server is down, use the
                    // other one" is a decision the agent can make and this
                    // code cannot.
                    Err(e) => (false, format!("ERROR: {e}")),
                };
                ctx.bus.publish(HiveEvent::ToolResult {
                    agent: agent_id.clone(),
                    label: label.clone(),
                    ok,
                    text: text.clone(),
                });
                exchanges.push(toolloop::exchange(&label, &call.args, &text));
                round += 1;
                prompt = toolloop::follow_up(&base, &exchanges, MAX_TOOL_ROUNDS - round);
            }
        })
    }
}

// ---------------------------------------------------------------------------
// ApiFactory
// ---------------------------------------------------------------------------

use crate::agent::AgentFactory;

/// Agent factory making native [`ApiAgent`]s that share one provider. Each
/// agent reads its model tier from its task at run time (see [`ApiAgent::run`]),
/// so the factory only needs the provider and the per-task output token cap.
pub struct ApiFactory {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
    /// Shared by every agent the factory makes, so one MCP host and ONE
    /// approval gate serve the whole swarm. Handing each agent its own would
    /// mean a person approving the same irreversible tool once per agent.
    tools: Option<Arc<dyn Tools>>,
}

impl ApiFactory {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
            tools: None,
        }
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

impl AgentFactory for ApiFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        let mut agent = ApiAgent::new(Arc::clone(&self.provider), self.max_tokens);
        if let Some(m) = &self.model {
            agent = agent.with_model(m.clone());
        }
        if let Some(t) = &self.tools {
            agent = agent.with_tools(Arc::clone(t));
        }
        Box::new(agent)
    }
}
