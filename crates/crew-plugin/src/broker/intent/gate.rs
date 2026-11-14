//! The deterministic HUMAN GATES, checked before any model call: a pending
//! commit proposal plus the user's own confirm word creates the commit, and a
//! pending plan plus the user's own verdict word runs or discards it. Each is
//! an exact match against a small fixed synonym set (trimmed, lowercased,
//! trailing punctuation stripped) — a misclassification can draft, but it can
//! never apply, run, or drop anything.
//!
//! [`human_gates`] is the ORDER, in one place: everything the router settles
//! by reading the message rather than by asking a model.
use crate::PluginEvent;

use super::super::session::Session;

/// Every gate, in the order that makes them safe, or `None` when the message
/// is an ordinary task. Undo leads: it is the only one that writes over your
/// files, so a "yes" it is holding must never reach a draft instead. Then the
/// session switches (plan-first, the memory question — neither touches
/// anything), then commit — on the overlapping confirm words ("yes", "do it")
/// a pending commit outranks a pending plan — then the plan verdict, then a
/// review of the working changes. Anything else falls through and the draft
/// stays pending.
pub(crate) fn human_gates(
    task: &str,
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> Option<anyhow::Result<()>> {
    if let Some(done) = crate::broker::undo::gate(task, session, emit) {
        return Some(done);
    }
    if let Some(done) = crate::broker::planfirst::gate(task, session, emit) {
        return Some(done);
    }
    if let Some(done) = crate::broker::recallask::gate(task, session, emit) {
        return Some(done);
    }
    if let Some(done) = crate::broker::ruledask::gate(task, session, emit) {
        return Some(done);
    }
    if let Some(done) = crate::broker::selfrepair::gate(task, session, emit) {
        return Some(done);
    }
    if confirms_apply(task) && pending_commit(session) {
        return Some(crate::broker::gitmsg::commit_cmd(session, "apply", emit));
    }
    if pending_plan(session) {
        if approves_plan(task) {
            return Some(crate::broker::plan::approve_cmd(session, emit));
        }
        if rejects_plan(task) {
            return Some(crate::broker::plan::reject_cmd(session, emit));
        }
    }
    // Last, and only when nothing above held the message: it reads but
    // writes nothing (`reviewask`, `handsoff`).
    if crate::broker::reviewask::asks_review(task) {
        return Some(crate::broker::review::review_cmd(session, emit));
    }
    None
}

/// A turn with a lead still passes the gates.
///
/// With a CLI provider signed in (Claude Code on a subscription), a plain
/// message skips the router and goes to that CLI as the turn's lead
/// (`auth::starter`). The gates in front of the router went with it: "undo
/// that" became a task for Claude, and "review my diff" an open task that a
/// peer answered by editing the callers it should have reported (loop 39
/// S8). The gates read the message rather than ask a model, so they cost the
/// lead nothing; they run first here as they do in [`super::routed`].
///
/// So: `task` through the human gates, then to `lead` as `@lead task` would go.
pub(crate) fn led(
    task: &str,
    lead: &str,
    session: &mut Session,
    tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if let Some(done) = human_gates(task, session, emit) {
        return done;
    }
    // No router reads a led turn, so the openings decide (`correction`).
    crate::broker::correction::note(&session.recall, None, task);
    let dialled = format!("@{lead} {task}");
    super::super::stdio::relay_counting(&dialled, None, false, session, tick_emit, emit)
}

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

/// The confirm and reject vocabulary for a gate that lives outside this
/// module (`crate::broker::undo`): one set of yes-words and no-words for
/// every pending thing in the pane, so "yes" never means one thing to a
/// commit draft and another to an undo offer.
pub(crate) fn confirm_word(task: &str) -> bool {
    confirms_apply(task) || approves_plan(task)
}

pub(crate) fn reject_word(task: &str) -> bool {
    rejects_plan(task)
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
