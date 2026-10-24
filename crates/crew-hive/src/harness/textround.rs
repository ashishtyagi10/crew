//! One `@tool` round of a text-mode worker: every call its reply ends with.
//!
//! The tools hint says several reads may go on consecutive last lines, and the
//! worker took only the last of them (`split_tool_call`), so one that wrote
//! three reads got one result and silently lost the other two. Now the reply
//! is one round — a round is a round trip, as the relay counted it: counted
//! per call, a rename's six edits spent a solo turn's budget before its last
//! file (live, 2026-10-20) — and its calls, as far as the per-reply bound,
//! run as one batch (`pending`: together when all only read), each its own
//! exchange in the next prompt, the reply's message written once, with the
//! first.

use std::sync::Arc;

use super::native::{past_bound, MAX_CALLS_PER_TURN};
use super::pending::{self, Rules};
use super::toolloop;
use super::TurnContext;
use crate::bus::HiveEvent;
use crate::tools::exchanges::Exchanges;
use crate::tools::{seen::Seen, ToolCall, Tools};

/// Run `calls`, the ones one reply asked for, and record them in `log` as
/// one round, `said` (what the reply wrote above them) with the first. A call
/// past [`MAX_CALLS_PER_TURN`] is answered without running, in the log and
/// to the pane, so a model that wrote twenty reads learns why it got eight.
/// Returns how each call that ran came back, `(ok, text)`, in order.
///
/// Each is noted in `seen` under the exchange it becomes, so a repeat points
/// at the one call it repeats, and only while that call is still shown whole
/// (`Exchanges::repeat`).
pub(super) async fn run(
    ctx: &TurnContext,
    tools: &Arc<dyn Tools>,
    seen: &mut Seen,
    log: &mut Exchanges,
    said: &str,
    calls: &[ToolCall],
) -> Vec<(bool, String)> {
    let (run, over) = calls.split_at(calls.len().min(MAX_CALLS_PER_TURN));
    let next = log.next_entry();
    let shown: &Exchanges = log;
    let rules = Rules {
        entry: &|k| next + k as u32,
        repeat: &|seen: &Seen, c: &ToolCall| shown.repeat(seen, c),
        error: |e| format!("ERROR: {e}"),
    };
    let ran = pending::run(ctx, tools, seen, run, rules).await;
    let refusal = past_bound();
    for call in over {
        ctx.publish(HiveEvent::ToolResult {
            agent: ctx.agent.clone(),
            label: call.label(),
            ok: false,
            text: refusal.clone(),
            ms: 0,
        });
    }
    let texts = ran
        .iter()
        .map(|(_, text)| text.as_str())
        .chain(std::iter::repeat_n(refusal.as_str(), over.len()));
    let round = calls
        .iter()
        .zip(texts)
        .enumerate()
        .map(|(k, (call, text))| {
            let said = if k == 0 { said } else { "" };
            toolloop::exchange(said, &call.label(), &call.args, text)
        })
        .collect();
    log.push_round(round);
    ran
}

#[cfg(test)]
#[path = "textround_tests.rs"]
mod tests;
