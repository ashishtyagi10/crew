//! Plan-first: the session mode where nothing runs until you have read the
//! plan and said so.
//!
//! Claude Code has it on a key (shift-tab), Codex has it as a mode flag, and
//! both are answering the same want: *think before you touch anything*, for a
//! stretch of work rather than for one message. Crew had the shape already —
//! "draft a plan for …" routes to the plan gate, which waits for your
//! "approve" — but it was a per-message phrasing, so on a run of related
//! tasks you had to remember to ask for it every single time, and the one you
//! forgot was the one that edited four files.
//!
//! So it is a MODE now, turned on in words ("plan first") and off in words
//! ("stop planning"), and while it is on every plain task is drafted as a
//! plan and waits. Nothing else changes: the same plan gate, the same
//! approve/reject words, the same swarm running the approved plan.
//!
//! Matched deterministically, like every other switch that changes what a
//! message DOES, so turning the mode off can never be mistaken for a task and
//! turning it on costs no model call.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::PluginEvent;

use super::relay::msg;
use super::session::Session;

/// The session's switch, shared with worker snapshots like every other bit
/// of session state a task reads.
pub(crate) type SharedPlanFirst = Arc<AtomicBool>;

/// What a message asked of the mode, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ask {
    On,
    Off,
}

/// Whether `task` is turning the mode on or off. Exact matches, trimmed and
/// lowercased: "plan first, then write the migration" is a TASK.
pub(crate) fn asks(task: &str) -> Option<Ask> {
    let t = task
        .trim()
        .trim_end_matches(['.', '!'])
        .to_ascii_lowercase();
    const ON: &[&str] = &[
        "plan first",
        "plan mode",
        "plan mode on",
        "always plan first",
        "plan before you act",
    ];
    const OFF: &[&str] = &[
        "stop planning",
        "plan mode off",
        "no more plans",
        "stop plan mode",
        "just do it from now on",
    ];
    if ON.contains(&t.as_str()) {
        return Some(Ask::On);
    }
    OFF.contains(&t.as_str()).then_some(Ask::Off)
}

pub(crate) fn on(session: &Session) -> bool {
    session.plan_first.load(Ordering::Relaxed)
}

/// The router's one call: answer a request to change the mode, or nothing.
pub(crate) fn gate(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> Option<anyhow::Result<()>> {
    let ask = asks(task)?;
    session.plan_first.store(ask == Ask::On, Ordering::Relaxed);
    Some(emit(msg("agent smith", line(ask))))
}

fn line(ask: Ask) -> &'static str {
    match ask {
        Ask::On => {
            "plan first is ON \u{2014} every task is drafted as a plan and waits for \
                    your \u{201c}approve\u{201d} (say \u{201c}stop planning\u{201d} to end it)"
        }
        Ask::Off => "plan first is OFF \u{2014} tasks run as agent smith routes them",
    }
}

/// The routing line's reason while the mode is on, so a forced plan never
/// looks like the classifier's own idea.
pub(crate) const WHY: &str = "plan first is on";

#[cfg(test)]
#[path = "planfirst_tests.rs"]
mod tests;
