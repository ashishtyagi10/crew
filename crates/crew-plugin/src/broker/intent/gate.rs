//! The deterministic HUMAN GATES, checked before any model call: a pending
//! commit proposal plus the user's own confirm word creates the commit, and a
//! pending plan plus the user's own verdict word runs or discards it. Each is
//! an exact match against a small fixed synonym set (trimmed, lowercased,
//! trailing punctuation stripped) — a misclassification can draft, but it can
//! never apply, run, or drop anything.
use super::super::session::Session;

/// Whether `task` is a conversational confirm for the pending commit.
/// Deliberately narrow: anything else is a new task, and the proposal simply
/// stays pending.
pub(super) fn confirms_apply(task: &str) -> bool {
    [
        "apply",
        "apply it",
        "yes",
        "yes apply",
        "yes, apply",
        "go ahead",
        "do it",
        "commit it",
        "ship it",
    ]
    .contains(&normalize(task).as_str())
}

/// Whether `task` approves the pending plan. Overlaps with the commit
/// confirms on purpose ("yes", "go ahead", "do it"): when both a commit and a
/// plan are pending, the commit gate is checked first and wins — an order the
/// router states, not an accident.
pub(super) fn approves_plan(task: &str) -> bool {
    [
        "approve",
        "approved",
        "yes",
        "go",
        "go ahead",
        "run it",
        "do it",
        "run the plan",
    ]
    .contains(&normalize(task).as_str())
}

/// Whether `task` rejects the pending plan.
pub(super) fn rejects_plan(task: &str) -> bool {
    [
        "reject",
        "rejected",
        "no",
        "drop it",
        "discard",
        "discard it",
        "never mind",
    ]
    .contains(&normalize(task).as_str())
}

fn normalize(task: &str) -> String {
    task.trim()
        .trim_end_matches(['.', '!'])
        .to_ascii_lowercase()
}

/// Whether the session holds a drafted commit message awaiting the confirm.
pub(super) fn pending_commit(session: &Session) -> bool {
    session
        .commit
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

/// Whether the session holds a drafted plan awaiting the verdict.
pub(super) fn pending_plan(session: &Session) -> bool {
    session
        .plan
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}
