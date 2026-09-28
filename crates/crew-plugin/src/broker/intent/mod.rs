//! Intent router — the model decides the execution shape of a plain message.
//!
//! Except where it must not: putting files back ("undo that") is matched
//! deterministically ahead of every model call, offered with a preview, and
//! applied only on the user's own confirm word (`super::undo`).
//!
//! A plain (non-slash, non-`@`) message used to go straight to the swarm.
//! Here one cheap completion classifies it into a shape first — `reply`,
//! `fan`, `loop`, `plan` or `swarm` — and dispatch reuses the EXISTING
//! capability paths (relay, fan-out, loop rounds, plan gate, swarm), so every
//! guard those paths enforce (hop cap, token budget, tool rounds) applies
//! unchanged. A classifier that may not run — `CREW_INTENT=0`, no API key,
//! the mock provider — leaves the pre-router behavior: the swarm. One that
//! runs and stumbles — a failed call, a reply off-grammar twice — lands on a
//! single agent's reply, the cheap common case (`decision::Routing`).
//! Whatever it decides, the pane is told (see `decision`). The
//! model also sizes the work (`hints`: rounds, a fan subset) and sees the
//! room it routes in (`world`: roster, dirty tree, tools); the round
//! constants here and in `constructs` are backstops, not the drivers.
use std::sync::Arc;

use crew_hive::Spent;

use crate::PluginEvent;

use super::session::Session;

mod classify;
mod context;
pub(crate) mod decision;
mod fanout;
pub(crate) mod gate;
mod hints;
mod shapeguard;
mod skillhint;
mod world;

pub(crate) use classify::{live_call, live_call_at, live_classifier, live_provider_at};
pub(crate) use fanout::fan_recorded;
pub(crate) use hints::Hints;
pub(crate) use world::World;

/// Relay rounds when the router picks `loop` and the model gave no
/// `ROUNDS:` — a modest backstop well inside `roundloop::MAX_ROUNDS`.
pub(crate) const LOOP_ROUNDS: u32 = 3;

/// A classification call: full prompt in, raw model reply out. A borrowed
/// closure so tests inject a deterministic, keyless model.
pub(crate) type Classifier<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// The router's call as it runs live: the reply, and what it cost. The cost
/// is the turn's — [`routed`] adds it to the total the arm says.
pub(crate) type SpentClassifier<'a> = &'a dyn Fn(&str) -> Result<(String, Spent), String>;

/// Where a routed message goes once its shape is said: [`dispatch`] in
/// production; a test hands in a swarm on scripted parts, so the routing
/// half runs for real around it.
pub(crate) type Arm<'a> = &'a mut dyn FnMut(
    &decision::Decision,
    &mut Session,
    &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()>;

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
    /// Decompose into a task graph (the pre-router default, and still where
    /// a classifier that may not run lands).
    #[default]
    Swarm,
    /// Draft a commit message for the working diff (the `/commit` body).
    /// Drafts ONLY: creating the commit takes the user's own "apply", matched
    /// deterministically in [`routed`] — never by classification.
    Commit,
    /// Code-review the working diff (the `/review` body).
    Review,
    /// Summarize recent commits as a standup update (the `/standup` body).
    Standup,
    /// Fold the previous session into the next task (the `/resume` body).
    Resume,
}

/// Route one plain message: classify, then dispatch. Disabled, keyless and
/// mock land on [`Shape::Swarm`], exactly the pre-router behavior; a call
/// error or an off-grammar reply lands on [`Shape::Reply`] (see
/// `decision::Routing::decision` for why the two differ).
pub(crate) fn route(
    task: &str,
    session: &mut Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let router = classify::live_router();
    let router = router.as_ref().map(|c| c as SpentClassifier);
    route_counted(task, router, session, tick_emit, emit)
}

/// [`route`] with the classifier passed in — the seam the parity tests use to
/// prove a plain phrasing reaches a capability whose slash command retired.
/// Its classifier reports no cost, so the arm's totals pass as they are.
#[cfg(test)]
pub(crate) fn route_with(
    task: &str,
    classifier: Option<Classifier>,
    session: &mut Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let counted = classifier.map(uncounted);
    let counted = counted.as_ref().map(|c| c as SpentClassifier);
    route_counted(task, counted, session, tick_emit, emit)
}

/// [`route`] on a router that says what it cost, dispatched for real — the
/// seam a test hands a costed router to.
pub(crate) fn route_counted(
    task: &str,
    classifier: Option<SpentClassifier>,
    session: &mut Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut arm = |d: &decision::Decision,
                   s: &mut Session,
                   emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>| {
        dispatch(d.shape, &d.hints, task, s, tick_emit, emit)
    };
    routed(task, classifier, session, &mut arm, emit)
}

/// A classifier that reports no cost, as the live router's shape — how the
/// tests' plain closures reach the counted path.
#[cfg(test)]
pub(crate) fn uncounted<'a>(
    call: Classifier<'a>,
) -> impl Fn(&str) -> Result<(String, Spent), String> + 'a {
    move |p: &str| call(p).map(|reply| (reply, Spent::default()))
}

/// Classify, say the decision, then run `arm` — the router's own cost folded
/// into the turn total the arm says (`turnspent`), since the router is a
/// call of the turn it routes and no arm counts it.
pub(crate) fn routed(
    task: &str,
    classifier: Option<SpentClassifier>,
    session: &mut Session,
    arm: Arm,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if let Some(done) = gate::human_gates(task, session, emit) {
        return done;
    }
    // Classify in the world the session can see, AND say so — the routing
    // line, then what the run brings (`context`), both before the arm's
    // first event, so the pane never has to guess why it got what it got.
    let world = World::gather_about(session, task);
    // Plan-first skips the classifier entirely: the shape is already decided,
    // and asking a model to choose one it cannot have is a call for nothing.
    let (routing, spent) = match super::planfirst::on(session) {
        true => (
            decision::forced(Shape::Plan, super::planfirst::WHY),
            Spent::default(),
        ),
        false => decision::classify_live(task, &world, classifier, emit)?,
    };
    let d = routing.decision();
    if let Some(names) = &d.hints.skills {
        skillhint::seed(task, names);
    }
    decision::say(
        &routing,
        context::words(d.shape, task, session, &world),
        emit,
    )?;
    super::turnspent::folded(spent, emit, |emit| arm(&d, session, emit))
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
        Shape::Reply => {
            let pick = hints.agents.as_ref().and_then(|a| a.first());
            let pick = pick.map(String::as_str);
            super::stdio::relay_counting(task, pick, true, session, tick_emit, emit)
        }
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
        Shape::Swarm => super::swarm::run_task_on(
            task,
            hints.verify,
            super::swarm::swarmconf::swarmtier::effective(hints.tier),
            session,
            emit,
        ),
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
///
/// Markdown around the line is not a different answer: a smaller router
/// (the cheap tier is qwen-flash on DashScope) now and then fences its reply
/// in ``` or bolds the head (`**SHAPE:** reply`), and that used to send the
/// task down the swarm fallback as "off-grammar". Fence lines are skipped and
/// emphasis is shed; the grammar itself is as strict as ever.
pub(crate) fn parse_shape(reply: &str) -> Option<Shape> {
    let first = reply
        .trim()
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("```"))?;
    let (head, tail) = first.split_once(':')?;
    let bare = |s: &str| {
        s.trim()
            .trim_matches(|c: char| matches!(c, '*' | '_' | '`'))
            .trim()
            .to_string()
    };
    if !bare(head).eq_ignore_ascii_case("shape") {
        return None;
    }
    let tail = tail.trim_start_matches(|c: char| matches!(c, '*' | '_' | '`'));
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
