//! The native tool-use loop: structured calls on the wire, no `@tool` line.
//!
//! Chosen over [`super::toolloop`]'s text convention whenever the tool surface
//! has schemas AND the provider speaks tools. The difference that matters is
//! not elegance: the model is shown each tool's JSON Schema and the provider
//! validates the arguments, instead of being shown a name and a 100-character
//! description clip and asked to hand-write JSON on the last line of a reply.

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;

#[path = "badargs.rs"]
mod badargs;
#[path = "batch.rs"]
mod batch;

use std::sync::Arc;

use crate::agent::{AgentContext, Attempt};
use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::provider::{CompletionRequest, Provider, ToolInvocation, ToolOutcome, Turn};
use crate::tools::{seen::Seen, ToolCatalog, Tools};

/// Most tools one turn may fire, however many the model asked for.
///
/// Parallel tool use is a real feature — three independent lookups in one turn
/// is exactly what a swarm wants — but "however many it emitted" is not a
/// bound. A model that emits fifty calls in one turn would run fifty shell
/// commands before anything counted a round.
pub(super) const MAX_CALLS_PER_TURN: usize = 8;

/// What a call resolved to, or why it did not.
fn outcome_for_unknown(call: &ToolInvocation, catalog: &ToolCatalog) -> ToolOutcome {
    let known = catalog.names();
    // Naming the near misses turns an invented name into one recoverable
    // round instead of a repeat of the same wrong guess — and at retrieval
    // scale the whole list would be the wall the picker exists to avoid.
    let tail = match known.contains(&"sys__find_tools") {
        true => "; sys__find_tools searches them",
        false => "",
    };
    ToolOutcome {
        id: call.id.clone(),
        name: call.name.clone(),
        content: crate::tools::near::unknown("tool", &call.name, &known, tail),
        is_error: true,
    }
}

/// Run one task with native tool use. Returns the task's attempt; every model
/// call, tool call and failure publishes on `ctx.bus` as it happens.
#[allow(clippy::too_many_arguments)] // one call site: `ApiAgent::attempt`'s native branch
pub(super) async fn run(
    ctx: AgentContext,
    provider: Arc<dyn Provider>,
    tools: Arc<dyn Tools>,
    catalog: ToolCatalog,
    model_id: String,
    system: Option<String>,
    prompt: String,
    max_tokens: u32,
    sink: super::chunks::ChunkSink,
) -> Attempt {
    let task_id = ctx.task.id;
    let agent_id = ctx.agent.clone();
    let tier = ctx.task.model;
    let mut turns: Vec<Turn> = Vec::new();
    let mut round: u32 = 0;
    // Reads this task has made. A repeat is answered from the result already
    // in `turns`, which every request resends whole. So `Seen::check` alone,
    // not `Exchanges::repeat`: the result a pointer names is always in the
    // request beside it, until the turns are cut to fit (`overflow::Cut`).
    let mut seen = Seen::default();
    let label = |c: &ToolInvocation| label_of(c, &catalog);
    let mut cut = super::overflow::Cut::new(&prompt, system.as_deref(), &label);

    loop {
        // A stop pressed while the last round's tools ran: no next call
        // (`agent::stop`). The tools already run are not undone.
        if ctx.stopped() {
            return Attempt::stopped(task_id);
        }
        cut.sending(&turns);
        let req = CompletionRequest {
            model: model_id.clone(),
            system: system.clone(),
            prompt: prompt.clone(),
            max_tokens,
            turns: turns.clone(),
            tools: catalog.defs().to_vec(),
        };
        // Raced against the stop: dropping the call aborts the request.
        let call = provider.complete_streaming(req, Arc::clone(&sink.on_chunk));
        let completion = match ctx.unless_stopped(call).await {
            None => return Attempt::stopped(task_id),
            Some(Ok(c)) => c,
            Some(Err(err)) if cut.refused(&ctx, &err, &mut turns, &mut seen) => continue,
            Some(Err(err)) => return super::failure::failed(&ctx, &err),
        };
        cut.answered();
        sink.settle(&completion);
        ctx.bus.publish(HiveEvent::TokenDelta {
            agent: agent_id.clone(),
            input: completion.input_tokens,
            output: completion.output_tokens,
        });
        ctx.bus.publish(HiveEvent::CostDelta {
            agent: agent_id.clone(),
            micros_usd: super::cost::billed(&model_id, tier, &completion),
        });

        if completion.calls.is_empty() {
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
        }

        if ctx.take_round(round).is_none() {
            // Unlike the text path there is no directive to strip — a
            // structured call never lands in the output — so the answer is
            // whatever the model said, plus a note that it stopped early.
            let total = ctx.budget.total();
            for call in &completion.calls {
                ctx.bus.publish(HiveEvent::ToolResult {
                    agent: agent_id.clone(),
                    label: label_of(call, &catalog),
                    ok: false,
                    text: format!("not run \u{2014} tool budget spent ({total} rounds this run)"),
                    ms: 0,
                });
            }
            // Everything gathered goes back once more as plain text with no
            // tools on the wire, for an answer (`lastword`); failing that,
            // what it had written beside the ask is all there is.
            let asked = completion
                .calls
                .iter()
                .map(|c| (label(c), c.input.to_string()));
            let last = super::lastword::prompt(
                &prompt,
                &super::lastword::transcript(&turns, label),
                &super::lastword::refused(&completion.text, asked),
            );
            let answer =
                super::lastword::ask(&ctx, &provider, &model_id, system, last, max_tokens, &sink)
                    .await;
            if ctx.stopped() {
                return Attempt::stopped(task_id);
            }
            let text = super::toolloop::with_budget_note(
                answer.as_deref().unwrap_or(&completion.text),
                total,
            );
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

        let results = batch::run(&ctx, &tools, &catalog, &mut seen, &completion, round).await;

        turns.push(Turn::Assistant {
            text: completion.text,
            calls: completion.calls,
        });
        turns.push(Turn::ToolResults(results));
        cut.after_round(&mut turns, &mut seen);
        round += 1;
    }
}

/// `server:tool` when the name resolves, else the name the model invented.
fn label_of(call: &ToolInvocation, catalog: &ToolCatalog) -> String {
    match catalog.resolve(&call.name) {
        Some((s, t)) => format!("{s}:{t}"),
        None => call.name.clone(),
    }
}
