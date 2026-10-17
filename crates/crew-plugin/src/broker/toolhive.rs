//! The API engine's tool calls, sent the way Claude Code's are.
//!
//! A relay hop's own runtime (Claude Code) reports its tools as hive events,
//! and the pane draws them in the agent's tool block: one live line a call,
//! folded to a single summary line when the reply settles. crew's own tool
//! loop sent each call and each result as a transcript card instead, `engineer
//! → sys:run · 13m ago` over the command and `sys:run → engineer` over its
//! output, so a four-call fix put sixteen rows of machinery between the
//! question and the answer (the user, 2026-10-15: no tool-call list). Now
//! both engines take the same lane (`HopStream::on_tool` → `hoptool`), and
//! the pane cannot tell them apart.
use crew_hive::tools::ToolCall;
use crew_hive::{AgentId, HiveEvent};

use super::adapter::HopStream;
use super::toolbatch::Ran;
use super::Envelope;

/// Chars of a result the tool line keeps for its click-open view.
const RESULT_KEEP: usize = 2_000;

/// `call`, as it is made.
pub(super) fn call(stream: &HopStream, env: &Envelope, call: &ToolCall) {
    (stream.on_tool)(HiveEvent::ToolCall {
        agent: AgentId::minted(&env.to),
        label: call.label(),
        args: call.args.clone(),
    });
}

/// `call`'s result: whether it worked, how long it took, and the head of
/// what it said. Only for a call that was made: one past the budget never
/// opened a line, and the turn's `[stopped]` note says why it stopped.
pub(super) fn result(stream: &HopStream, env: &Envelope, call: &ToolCall, ran: &Ran) {
    let mut text: String = ran.text.trim_end().chars().take(RESULT_KEEP).collect();
    if ran.text.trim_end().chars().count() > RESULT_KEEP {
        text.push_str("\n\u{2026}");
    }
    (stream.on_tool)(HiveEvent::ToolResult {
        agent: AgentId::minted(&env.to),
        label: call.label(),
        ok: ran.ok,
        text,
        ms: ran.ms,
    });
}

/// For tests of the tool loop: a stream whose tool lane keeps what it is
/// handed, and those events read back in the shape the cards had.
#[cfg(test)]
pub(crate) mod lane {
    use std::sync::{Arc, Mutex};

    use crew_hive::HiveEvent;

    use crate::broker::adapter::HopStream;

    pub(crate) type Log = Arc<Mutex<Vec<HiveEvent>>>;

    pub(crate) fn capture() -> (Log, HopStream) {
        let log: Log = Arc::default();
        let keep = log.clone();
        let mut s = HopStream::noop();
        s.on_tool = Arc::new(move |e| keep.lock().unwrap().push(e));
        (log, s)
    }

    /// `[tool] label  args` for a call, `[tool] label ✓|✗` over the text
    /// for a result.
    pub(crate) fn cards(log: &Log) -> Vec<String> {
        let mark = |ok: bool| if ok { '\u{2713}' } else { '\u{2717}' };
        let card = |e: &HiveEvent| match e {
            HiveEvent::ToolCall { label, args, .. } => Some(format!("[tool] {label}  {args}")),
            HiveEvent::ToolResult {
                label, ok, text, ..
            } => Some(format!("[tool] {label} {}\n{text}", mark(*ok))),
            _ => None,
        };
        log.lock().unwrap().iter().filter_map(card).collect()
    }
}

#[cfg(test)]
#[path = "toolhive_tests.rs"]
mod tests;
