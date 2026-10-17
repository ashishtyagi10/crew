//! A checklist an API agent wrote this round, sent to the pane as its plan.
//!
//! `sys:todo` is answered like any tool, so the model reads its list back.
//! The user saw it as two tool cards, `[tool] sys:todo {"items":[…]}` and
//! the list echoed, and nothing drew it as a plan. Now an accepted write goes
//! out the way Claude Code's TodoWrite does (`hoptool` turns both into
//! `PluginEvent::Todo`), and it makes no card of its own.
use crew_hive::tools::{todo, ToolCall};
use crew_hive::{AgentId, HiveEvent};

use super::adapter::HopStream;
use super::toolbatch::Ran;

/// Whether `call` gets the pane's tool cards: everything but a checklist
/// write, which the pane draws as the plan instead.
pub(super) fn carded(call: &ToolCall) -> bool {
    !todo::is_todo(call)
}

/// An accepted checklist write goes out as the plan. A refused one changed
/// nothing (the old list stands), so nothing goes out for it.
pub(super) fn send(stream: &HopStream, agent: &str, call: &ToolCall, ran: &Ran) {
    if todo::is_todo(call) && ran.ok {
        (stream.on_tool)(HiveEvent::ToolCall {
            agent: AgentId::minted(agent),
            label: call.label(),
            args: call.args.clone(),
        });
    }
}

#[cfg(test)]
#[path = "planout_tests.rs"]
mod tests;
