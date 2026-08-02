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
use std::time::Duration;

use crate::PluginEvent;

use super::session::Session;

/// Output-token ceiling for the classification call: the grammar is one line.
const INTENT_MAX_TOKENS: u32 = 64;

/// Round-trip ceiling for classification — deliberately far below
/// `call_timeout()` (3 min): the router is overhead before the real work, so
/// a slow classifier must degrade to the swarm, not stall the task.
const CLASSIFY_TIMEOUT: Duration = Duration::from_secs(30);

/// Relay rounds when the router picks `loop` — the user never typed a count,
/// so a modest default well inside `constructs::MAX_ROUNDS`.
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
    let shape = match live_classifier() {
        Some(call) => classify_with(task, &call).unwrap_or(Shape::Swarm),
        None => Shape::Swarm,
    };
    dispatch(shape, task, session, tick_emit, emit)
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
        Shape::Fan => super::commands::fan_cmd(session, task, tick_emit, emit),
        Shape::Loop => {
            super::constructs::loop_cmd(session, &format!("{LOOP_ROUNDS} {task}"), tick_emit, emit)
        }
        Shape::Plan => super::plan::plan_cmd(session, task, emit),
        Shape::Swarm => super::swarm::run_task(task, session, emit),
    }
}

/// Classify `task` through `call`: `None` on a call error or a reply outside
/// the grammar — the caller decides the fallback.
pub(crate) fn classify_with(task: &str, call: Classifier) -> Option<Shape> {
    call(&prompt(task)).ok().and_then(|r| parse_shape(&r))
}

/// `CREW_INTENT=0` — the escape hatch back to the old always-swarm routing.
pub(crate) fn disabled() -> bool {
    std::env::var("CREW_INTENT").is_ok_and(|v| v == "0")
}

/// The live classifier, when one may run: `None` under `CREW_INTENT=0`, with
/// no resolvable provider, or under the mock provider (the GUI harness needs
/// deterministic swarm replies; a mock reply would fail the grammar anyway).
fn live_classifier() -> Option<impl Fn(&str) -> Result<String, String>> {
    if disabled() {
        return None;
    }
    let (provider, model) = super::discover::provider_and_model()?;
    if model == "mock" {
        return None;
    }
    Some(move |p: &str| complete_once(&provider, &model, p))
}

/// One bounded completion on the discovered provider — same block-on pattern
/// as `ask::suggest_far_command` (a small one-shot needs its own max_tokens,
/// which the `Adapter` layer doesn't expose).
fn complete_once(
    provider: &Arc<dyn crew_hive::Provider>,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    let req = crew_hive::CompletionRequest {
        model: model.to_string(),
        system: None,
        prompt: prompt.to_string(),
        max_tokens: INTENT_MAX_TOKENS,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let fut = provider.complete(req);
    match rt.block_on(async move { tokio::time::timeout(CLASSIFY_TIMEOUT, fut).await }) {
        Ok(Ok(c)) => Ok(c.text),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!(
            "intent classification timed out after {CLASSIFY_TIMEOUT:?}"
        )),
    }
}

/// The classification prompt: the five shapes and the reply grammar.
fn prompt(task: &str) -> String {
    format!(
        "You route a user's message to ONE execution shape:\n\
         reply — a single agent answers or does it directly in one turn\n\
         fan — every agent tackles the same thing independently (the user wants many takes)\n\
         loop — one result refined over several rounds (iterate/polish/keep improving)\n\
         plan — draft a plan for approval before anything runs\n\
         swarm — multi-part work worth decomposing into parallel tasks\n\
         The FIRST line of your reply must be exactly `SHAPE: <reply|fan|loop|plan|swarm>`.\n\n\
         Message: {task}"
    )
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
        _ => None,
    }
}

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
