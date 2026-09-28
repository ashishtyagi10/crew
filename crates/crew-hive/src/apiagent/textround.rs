//! One `@tool` round of a text-mode worker: every call its reply ends with.
//!
//! The tools hint says several reads may go on consecutive last lines, and the
//! worker took only the last of them (`split_tool_call`), so one that wrote
//! three reads got one result and silently lost the other two. Now each call
//! draws its own round from the run's pool, the ones granted run as one batch
//! (`pending`: together when all only read), and each becomes its own exchange
//! in the next prompt, the reply's message written once, with the first.

use std::sync::Arc;

use super::pending::{self, Rules};
use super::toolloop;
use crate::agent::AgentContext;
use crate::tools::exchanges::Exchanges;
use crate::tools::{seen::Seen, ToolCall, Tools};

/// Draw one round per call, in the order written, for an agent that has
/// drawn `used` already this task: how many calls got one, and how many
/// rounds are left after the last that did. The first refusal ends it, so the
/// pool is asked once for a round it cannot give, as it always was.
pub(super) fn draw(ctx: &AgentContext, used: &mut u32, calls: usize) -> (usize, u32) {
    let mut left = 0;
    for granted in 0..calls {
        match ctx.take_round(*used) {
            Some(n) => {
                *used += 1;
                left = n;
            }
            None => return (granted, left),
        }
    }
    (calls, left)
}

/// Run `calls`, the ones the budget granted, and record them in `log` as one
/// round, `said` (what the reply wrote above them) with the first.
///
/// Each is noted in `seen` under the exchange it becomes, so a repeat points
/// at the one call it repeats, and only while that call is still shown whole
/// (`Exchanges::repeat`).
pub(super) async fn run(
    ctx: &AgentContext,
    tools: &Arc<dyn Tools>,
    seen: &mut Seen,
    log: &mut Exchanges,
    said: &str,
    calls: &[ToolCall],
) {
    let next = log.next_entry();
    let shown: &Exchanges = log;
    let rules = Rules {
        entry: &|k| next + k as u32,
        repeat: &|seen: &Seen, c: &ToolCall| shown.repeat(seen, c),
        error: |e| format!("ERROR: {e}"),
    };
    let ran = pending::run(ctx, tools, seen, calls, rules).await;
    let round = calls
        .iter()
        .zip(ran)
        .enumerate()
        .map(|(k, (call, (_, text)))| {
            let said = if k == 0 { said } else { "" };
            toolloop::exchange(said, &call.label(), &call.args, &text)
        })
        .collect();
    log.push_round(round);
}

#[cfg(test)]
#[path = "textround_tests.rs"]
mod tests;
