//! Intent router — the model decides the execution shape of a plain message.
//!
//! A plain (non-slash, non-`@`) message used to go straight to the swarm.
//! Here one cheap completion classifies it into a shape first — `reply`,
//! `fan`, `loop`, `plan` or `swarm` — and dispatch reuses the EXISTING
//! capability paths (relay, fan-out, loop rounds, plan gate, swarm), so every
//! guard those paths enforce (hop cap, token budget, tool rounds) applies
//! unchanged. Anything that stops the classifier — `CREW_INTENT=0`, no API
//! key, the mock provider, a parse failure — falls back to today's behavior:
//! the swarm. Whatever it decides, the pane is told (see `decision`). The
//! model also sizes the work (`hints`: rounds, a fan subset) and sees the
//! room it routes in (`world`: roster, dirty tree, tools); the round
//! constants here and in `constructs` are backstops, not the drivers.
use std::sync::Arc;

use crate::PluginEvent;

use super::session::Session;

mod classify;
mod decision;
mod fanout;
mod gate;
mod hints;
mod world;

pub(crate) use classify::{live_call, live_classifier};
pub(crate) use fanout::fan_recorded;
pub(crate) use hints::Hints;
pub(crate) use world::World;

/// Relay rounds when the router picks `loop` and the model gave no
/// `ROUNDS:` — a modest backstop well inside `roundloop::MAX_ROUNDS`.
pub(crate) const LOOP_ROUNDS: u32 = 3;

/// A classification call: full prompt in, raw model reply out. A borrowed
/// closure so tests inject a deterministic, keyless model.
pub(crate) type Classifier<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// The execution shapes a plain message can take. Every variant dispatches to
/// a capability path that already exists — the router adds no execution logic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Shape {
    /// One agent answers directly (the relay).
    Reply,
    /// Every agent answers the same task in parallel (the fan-out body).
    Fan,
    /// Iterative refinement rounds (the loop body).
    Loop,
    /// Draft a plan and wait for approval (the plan body).
    Plan,
    /// Relay rounds until a judge agent rules a stated goal met (the `/goal`
    /// body — the round cap stays a backstop).
    Goal,
    /// Decompose into a task graph (today's default, and every stop's).
    #[default]
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
    // The HUMAN GATES (see `gate`), checked before any model call. Commit
    // first — on the overlapping confirm words ("yes", "do it") a pending
    // commit outranks a pending plan. Then the plan verdict: with a plan
    // pending, the user's own word runs or discards it; anything else falls
    // through and the plan stays pending.
    if gate::confirms_apply(task) && gate::pending_commit(session) {
        return super::gitmsg::commit_cmd(session, "apply", emit);
    }
    if gate::pending_plan(session) {
        if gate::approves_plan(task) {
            return super::plan::approve_cmd(session, emit);
        }
        if gate::rejects_plan(task) {
            return super::plan::reject_cmd(session, emit);
        }
    }
    // Classify in the world the session can see, AND say so — the routing
    // line lands before the arm's first event, so the pane never has to
    // guess why it got what it got.
    let world = World::gather(session);
    let d = decision::announce(task, &world, classifier, emit)?;
    dispatch(d.shape, &d.hints, task, session, tick_emit, emit)
}

/// Send `task` down `shape`'s existing capability path, sized by `hints`
/// where the shape has a size (loop/goal rounds, the fan subset, whether the
/// swarm's result is judged) and by the backstop constants otherwise. Each
/// arm is the same function the equivalent construct/relay route calls, so
/// the hop cap, token budget and tool-round guards all apply unchanged.
pub(crate) fn dispatch(
    shape: Shape,
    hints: &Hints,
    task: &str,
    session: &mut Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    match shape {
        Shape::Reply => super::stdio::relay_counting(task, session, tick_emit, emit),
        Shape::Fan => fanout::fan_cmd(session, task, hints.agents.as_deref(), tick_emit, emit),
        Shape::Loop => {
            let n = hints.rounds.unwrap_or(LOOP_ROUNDS);
            super::roundloop::loop_cmd(session, &format!("{n} {task}"), tick_emit, emit)
        }
        Shape::Plan => super::plan::plan_cmd(session, task, hints.verify, emit),
        Shape::Goal => {
            let n = hints.rounds.unwrap_or(super::constructs::GOAL_ROUNDS);
            super::constructs::goal_rounds(session, task, n, tick_emit, emit)
        }
        Shape::Swarm => super::swarm::run_task(task, hints.verify, session, emit),
        Shape::Commit => super::gitmsg::commit_cmd(session, "", emit),
        Shape::Review => super::review::review_cmd(session, emit),
        Shape::Standup => super::standup::standup_cmd(session, "", emit),
        Shape::Resume => super::sessionlog::resume_cmd(session, emit),
    }
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
        "goal" => Some(Shape::Goal),
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
