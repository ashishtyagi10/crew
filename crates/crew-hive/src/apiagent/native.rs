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

use std::sync::Arc;

use crate::agent::AgentContext;
use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::provider::{ChunkFn, CompletionRequest, Provider, ToolInvocation, ToolOutcome, Turn};
use crate::tools::{ToolCatalog, Tools, MAX_TOOL_ROUNDS};

/// Most tools one turn may fire, however many the model asked for.
///
/// Parallel tool use is a real feature — three independent lookups in one turn
/// is exactly what a swarm wants — but "however many it emitted" is not a
/// bound. A model that emits fifty calls in one turn would run fifty shell
/// commands before anything counted a round.
pub(super) const MAX_CALLS_PER_TURN: usize = 8;

/// What a call resolved to, or why it did not.
fn outcome_for_unknown(call: &ToolInvocation, catalog: &ToolCatalog) -> ToolOutcome {
    let mut known = catalog.names();
    known.sort_unstable();
    ToolOutcome {
        id: call.id.clone(),
        name: call.name.clone(),
        // Naming the real options turns an invented name into one recoverable
        // round instead of a repeat of the same wrong guess.
        content: format!(
            "unknown tool \u{201c}{}\u{201d} \u{2014} available: {}",
            call.name,
            known.join(", ")
        ),
        is_error: true,
    }
}

/// Run one task with native tool use. Returns the task's result; every model
/// call, tool call and failure publishes on `ctx.bus` as it happens.
#[allow(clippy::too_many_arguments)] // one call site: `ApiAgent::run`'s native branch
pub(super) async fn run(
    ctx: AgentContext,
    provider: Arc<dyn Provider>,
    tools: Arc<dyn Tools>,
    catalog: ToolCatalog,
    model_id: String,
    system: Option<String>,
    prompt: String,
    max_tokens: u32,
    on_chunk: ChunkFn,
) -> TaskResult {
    let task_id = ctx.task.id;
    let agent_id = ctx.agent.clone();
    let tier = ctx.task.model;
    let mut turns: Vec<Turn> = Vec::new();
    let mut round: u32 = 0;

    loop {
        let req = CompletionRequest {
            model: model_id.clone(),
            system: system.clone(),
            prompt: prompt.clone(),
            max_tokens,
            turns: turns.clone(),
            tools: catalog.defs().to_vec(),
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
        ctx.bus.publish(HiveEvent::TokenDelta {
            agent: agent_id.clone(),
            input: completion.input_tokens,
            output: completion.output_tokens,
        });
        ctx.bus.publish(HiveEvent::CostDelta {
            agent: agent_id.clone(),
            micros_usd: super::cost_micros(tier, completion.input_tokens, completion.output_tokens),
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
            };
        }

        if round >= MAX_TOOL_ROUNDS {
            // Unlike the text path there is no directive to strip — a
            // structured call never lands in the output — so the answer is
            // whatever the model said, plus a note that it stopped early.
            for call in &completion.calls {
                ctx.bus.publish(HiveEvent::ToolResult {
                    agent: agent_id.clone(),
                    label: label_of(call, &catalog),
                    ok: false,
                    text: format!("not run \u{2014} tool budget spent ({MAX_TOOL_ROUNDS} rounds)"),
                });
            }
            let text = super::toolloop::with_budget_note(&completion.text, MAX_TOOL_ROUNDS);
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

        let mut results = Vec::with_capacity(completion.calls.len());
        for (i, call) in completion.calls.iter().enumerate() {
            // EVERY call gets a result, including the ones refused for being
            // over the per-turn bound: providers reject a follow-up whose
            // tool_call ids are not all answered, so skipping one would fail
            // the next request rather than the call.
            if i >= MAX_CALLS_PER_TURN {
                results.push(ToolOutcome {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    content: format!(
                        "not run \u{2014} at most {MAX_CALLS_PER_TURN} tools per turn"
                    ),
                    is_error: true,
                });
                continue;
            }
            let Some((server, tool)) = catalog.resolve(&call.name) else {
                let o = outcome_for_unknown(call, &catalog);
                ctx.bus.publish(HiveEvent::ToolResult {
                    agent: agent_id.clone(),
                    label: call.name.clone(),
                    ok: false,
                    text: o.content.clone(),
                });
                results.push(o);
                continue;
            };
            let label = format!("{server}:{tool}");
            let args = call.input.to_string();
            ctx.bus.publish(HiveEvent::ToolCall {
                agent: agent_id.clone(),
                label: label.clone(),
                args: args.clone(),
            });
            // Off the runtime thread — see the note in `ApiAgent::run`; the
            // scheduler's agents and its bus drain share one thread.
            let runner = Arc::clone(&tools);
            let (s, t) = (server.to_string(), tool.to_string());
            let called = tokio::task::spawn_blocking(move || runner.call(&s, &t, &args))
                .await
                .unwrap_or_else(|e| Err(format!("tool task failed: {e}")));
            let (ok, text) = match called {
                Ok(v) if v.trim().is_empty() => (true, "(empty result)".to_string()),
                Ok(v) => (true, v),
                Err(e) => (false, e),
            };
            ctx.bus.publish(HiveEvent::ToolResult {
                agent: agent_id.clone(),
                label,
                ok,
                text: text.clone(),
            });
            results.push(ToolOutcome {
                id: call.id.clone(),
                name: call.name.clone(),
                content: super::toolloop::clip(&text, super::toolloop::RESULT_CAP),
                is_error: !ok,
            });
        }

        turns.push(Turn::Assistant {
            text: completion.text,
            calls: completion.calls,
        });
        turns.push(Turn::ToolResults(results));
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
