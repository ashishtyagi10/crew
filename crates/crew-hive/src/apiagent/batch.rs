//! One native reply's tool calls: named the way crew names them, run through
//! the shared batch (`pending`, all at once when every call only reads), and
//! answered one outcome per call, each paired to its call by id.

use std::sync::Arc;

use super::super::pending::{self, Rules};
use super::super::toolloop::{clip, RESULT_CAP};
use super::{label_of, outcome_for_unknown, MAX_CALLS_PER_TURN};
use crate::agent::AgentContext;
use crate::bus::HiveEvent;
use crate::provider::{Completion, ToolInvocation, ToolOutcome};
use crate::tools::{seen::Seen, ToolCall, ToolCatalog, Tools};

/// Run the tools `reply` asked for in round `round` (from 0), and hand back
/// one outcome per call in the order they were asked. Every call publishes
/// its own `ToolCall` and `ToolResult`, as before.
pub(super) async fn run(
    ctx: &AgentContext,
    tools: &Arc<dyn Tools>,
    catalog: &ToolCatalog,
    seen: &mut Seen,
    reply: &Completion,
    round: u32,
) -> Vec<ToolOutcome> {
    let calls = &reply.calls;
    let mut results: Vec<Option<ToolOutcome>> = vec![None; calls.len()];
    let (mut asked, mut at, mut refused) = (Vec::new(), Vec::new(), Vec::new());
    for (i, call) in calls.iter().enumerate() {
        match runnable(catalog, call, i, reply.truncated) {
            Ok(c) => {
                asked.push(c);
                at.push(i);
            }
            Err(o) => {
                refused.push((i, o.content.clone()));
                results[i] = Some(o);
            }
        }
    }
    // Every call of a native round is noted under the round: the whole turn
    // is resent each time, so any earlier result a pointer names is there.
    let rules = Rules {
        entry: &|_| round + 1,
        repeat: &|seen: &Seen, c: &ToolCall| seen.check(c),
        error: |e| e,
    };
    let ran = pending::run(ctx, tools, seen, &asked, rules).await;
    for (i, (ok, text)) in at.into_iter().zip(ran) {
        results[i] = Some(ToolOutcome {
            id: calls[i].id.clone(),
            name: calls[i].name.clone(),
            content: clip(&text, RESULT_CAP),
            is_error: !ok,
        });
    }
    // The person watching gets each refusal too: with no `ToolCall` for it,
    // this result is the ONLY trace the pane ever sees of the call (it lands
    // as a failed line, `chattool`). Published once every call that ran has
    // its result, because the pane closes the oldest open line with a
    // result's label, and a refusal sent while a call of the same name was
    // still running would have closed that one.
    for (i, text) in refused {
        ctx.bus.publish(HiveEvent::ToolResult {
            agent: ctx.agent.clone(),
            label: label_of(&calls[i], catalog),
            ok: false,
            text,
            ms: 0,
        });
    }
    results.into_iter().flatten().collect()
}

/// Call `i` of the reply as crew names it, or its answer when it will not
/// run: past the per-turn bound, a name nothing answers to, or arguments that
/// were not JSON (`badargs`; `cut`, the reply stopped at the token limit).
fn runnable(
    catalog: &ToolCatalog,
    call: &ToolInvocation,
    i: usize,
    cut: bool,
) -> Result<ToolCall, ToolOutcome> {
    // EVERY call gets a result, including the ones refused for being over the
    // per-turn bound: providers reject a follow-up whose tool_call ids are not
    // all answered, so skipping one would fail the next request rather than
    // the call.
    if i >= MAX_CALLS_PER_TURN {
        return Err(ToolOutcome {
            id: call.id.clone(),
            name: call.name.clone(),
            content: format!("not run \u{2014} at most {MAX_CALLS_PER_TURN} tools per turn"),
            is_error: true,
        });
    }
    let Some((server, tool)) = catalog.resolve(&call.name) else {
        return Err(outcome_for_unknown(call, catalog));
    };
    if let Some(refused) = super::badargs::refusal(call, cut) {
        return Err(refused);
    }
    Ok(ToolCall {
        server: server.to_string(),
        tool: tool.to_string(),
        args: call.input.to_string(),
    })
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
