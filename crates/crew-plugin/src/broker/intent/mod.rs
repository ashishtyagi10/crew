//! Intent router — the model decides the execution shape of a plain message.
//!
//! A plain (non-slash, non-`@`) message used to go straight to the swarm.
//! Here one cheap completion classifies it into a shape first — `reply`,
//! `fan`, `loop`, `plan` or `swarm` — and dispatch reuses the EXISTING
//! capability paths (relay, fan-out, loop rounds, plan gate, swarm), so every
//! guard those paths enforce (hop cap, token budget, tool rounds) applies
//! unchanged. Anything that stops the classifier — `CREW_INTENT=0`, no API
//! key, the mock provider, a parse failure — falls back to today's behavior:
//! the swarm.
use std::sync::Arc;

use crate::PluginEvent;

use super::session::Session;

mod classify;
mod fanout;

pub(crate) use classify::live_classifier;

/// Relay rounds when the router picks `loop` — the user never typed a count,
/// so a modest default well inside `roundloop::MAX_ROUNDS`.
pub(crate) const LOOP_ROUNDS: u32 = 3;

/// A classification call: full prompt in, raw model reply out. A borrowed
/// closure so tests inject a deterministic, keyless model.
pub(crate) type Classifier<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// The execution shapes a plain message can take. Every variant dispatches to
/// a capability path that already exists — the router adds no execution logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// One agent answers directly (the relay).
    Reply,
    /// Every agent answers the same task in parallel (the fan-out body).
    Fan,
    /// Iterative refinement rounds (the loop body).
    Loop,
    /// Draft a plan and wait for approval (the plan body).
    Plan,
    /// Decompose into a task graph (today's default).
    Swarm,
    /// Draft a commit message for the working diff (the `/commit` body).
    /// Drafts ONLY: creating the commit takes the user's own "apply", matched
    /// deterministically in [`route_with`] — never by classification.
    Commit,
    /// Code-review the working diff (the `/review` body).
    Review,
    /// Summarize recent commits as a standup update (the `/standup` body).
    Standup,
    /// Fold the previous session into the next task (the `/resume` body).
    Resume,
}

/// Route one plain message: classify, then dispatch. Every way the classifier
/// can stop — disabled, keyless, mock, call error, off-grammar reply — lands
/// on [`Shape::Swarm`], exactly the pre-router behavior.
pub(crate) fn route(
    task: &str,
    session: &mut Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    match classify::live_classifier() {
        Some(call) => route_with(task, Some(&call), session, tick_emit, emit),
        None => route_with(task, None, session, tick_emit, emit),
    }
}

/// [`route`] with the classifier passed in — the seam the parity tests use to
/// prove a plain phrasing reaches a capability whose slash command retired.
pub(crate) fn route_with(
    task: &str,
    classifier: Option<Classifier>,
    session: &mut Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // The commit HUMAN GATE, checked before any model call: a pending
    // proposal plus the user's own confirm word creates the commit. This is
    // the only path that ever commits, and it is deterministic — a
    // misclassification can draft a message but can never apply one.
    if confirms_apply(task) && pending_commit(session) {
        return super::gitmsg::commit_cmd(session, "apply", emit);
    }
    let shape = match classifier {
        Some(call) => classify_with(task, call).unwrap_or(Shape::Swarm),
        None => Shape::Swarm,
    };
    dispatch(shape, task, session, tick_emit, emit)
}

/// Whether `task` is a conversational confirm for the pending commit — an
/// exact match against a small fixed set, lowercased and stripped of trailing
/// punctuation. Deliberately narrow: anything else is a new task, and the
/// proposal simply stays pending.
fn confirms_apply(task: &str) -> bool {
    let word = task
        .trim()
        .trim_end_matches(['.', '!'])
        .to_ascii_lowercase();
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
    .contains(&word.as_str())
}

/// Whether the session holds a drafted commit message awaiting the confirm.
fn pending_commit(session: &Session) -> bool {
    session
        .commit
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

/// Send `task` down `shape`'s existing capability path. Each arm is the same
/// function the equivalent construct/relay route calls, so the hop cap, token
/// budget and tool-round guards all apply unchanged.
pub(crate) fn dispatch(
    shape: Shape,
    task: &str,
    session: &mut Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    match shape {
        Shape::Reply => super::stdio::relay_counting(task, session, tick_emit, emit),
        Shape::Fan => fanout::fan_cmd(session, task, tick_emit, emit),
        Shape::Loop => {
            super::roundloop::loop_cmd(session, &format!("{LOOP_ROUNDS} {task}"), tick_emit, emit)
        }
        Shape::Plan => super::plan::plan_cmd(session, task, emit),
        Shape::Swarm => super::swarm::run_task(task, session, emit),
        Shape::Commit => super::gitmsg::commit_cmd(session, "", emit),
        Shape::Review => super::review::review_cmd(session, emit),
        Shape::Standup => super::standup::standup_cmd(session, "", emit),
        Shape::Resume => super::sessionlog::resume_cmd(session, emit),
    }
}

/// Classify `task` through `call`: `None` on a call error or a reply outside
/// the grammar — the caller decides the fallback.
pub(crate) fn classify_with(task: &str, call: Classifier) -> Option<Shape> {
    call(&classify::prompt(task))
        .ok()
        .and_then(|r| parse_shape(&r))
}

/// `CREW_INTENT=0` — the escape hatch back to the old always-swarm routing.
pub(crate) fn disabled() -> bool {
    std::env::var("CREW_INTENT").is_ok_and(|v| v == "0")
}

/// Parse the reply's first line against the `SHAPE: <shape>` grammar
/// (case-insensitive; trailing punctuation on the token and any prose after
/// the first line are tolerated — same conservatism as
/// `constructs::parse_verdict`). Anything else is `None`, never a guess.
pub(crate) fn parse_shape(reply: &str) -> Option<Shape> {
    let first = reply.trim().lines().next().unwrap_or("").trim();
    let (head, tail) = first.split_once(':')?;
    if !head.trim().eq_ignore_ascii_case("shape") {
        return None;
    }
    let token = tail
        .split_whitespace()
        .next()?
        .trim_matches(|c: char| !c.is_ascii_alphabetic())
        .to_ascii_lowercase();
    match token.as_str() {
        "reply" => Some(Shape::Reply),
        "fan" => Some(Shape::Fan),
        "loop" => Some(Shape::Loop),
        "plan" => Some(Shape::Plan),
        "swarm" => Some(Shape::Swarm),
        "commit" => Some(Shape::Commit),
        "review" => Some(Shape::Review),
        "standup" => Some(Shape::Standup),
        "resume" => Some(Shape::Resume),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../intent_tests/mod.rs"]
mod tests;
