//! Model-elected agents. The goal judge and the commit/review/standup
//! authors used to be chosen by substring-matching an agent's role string
//! against capability keywords; that path is gone (goal condition 3).
//! The MODEL now picks from the live roster through one small
//! structured call — strict `AGENT: <name>` first-line grammar, the reply
//! validated against the roster — and every way the call can stop (keyless,
//! mock, off-grammar, error) lands on the deterministic fallback: the
//! roster's first eligible agent, so tests and keyless machines stay exactly
//! as predictable as before.
use crate::AgentInfo;

use super::intent::{live_classifier, Classifier};
use super::route::clip;

/// Elect the agent best suited to `purpose` with the live model, if one may
/// run. `exclude` is the worker whose output is being judged — electing it
/// would be grading its own homework, so it is only ever returned when the
/// roster holds nobody else.
pub(crate) fn elect(purpose: &str, agents: &[AgentInfo], exclude: Option<&str>) -> String {
    match live_classifier() {
        Some(call) => elect_with(purpose, agents, exclude, Some(&call)),
        None => elect_with(purpose, agents, exclude, None),
    }
}

/// [`elect`] with the call injected — the test seam, mirroring
/// `intent::route_with`.
pub(crate) fn elect_with(
    purpose: &str,
    agents: &[AgentInfo],
    exclude: Option<&str>,
    call: Option<Classifier>,
) -> String {
    call.and_then(|c| c(&prompt(purpose, agents, exclude)).ok())
        .and_then(|reply| parse_agent(&reply, agents))
        .filter(|name| agents.len() == 1 || exclude != Some(name.as_str()))
        .unwrap_or_else(|| fallback(agents, exclude))
}

/// The deterministic election: the roster's first agent that isn't the
/// worker, else the worker itself (single-agent roster), else empty — only
/// reachable with an empty roster, which every call site has already ruled
/// out via `reg.is_empty()`.
fn fallback(agents: &[AgentInfo], exclude: Option<&str>) -> String {
    agents
        .iter()
        .find(|a| exclude != Some(a.name.as_str()))
        .or_else(|| agents.first())
        .map(|a| a.name.clone())
        .unwrap_or_default()
}

/// Parse the reply's first line against the `AGENT: <name>` grammar (same
/// conservatism as `intent::parse_shape`) and validate the name against the
/// roster: anything else is `None`, never a guess. Returns the roster's own
/// casing of the name.
pub(crate) fn parse_agent(reply: &str, agents: &[AgentInfo]) -> Option<String> {
    let first = reply.trim().lines().next().unwrap_or("").trim();
    let (head, tail) = first.split_once(':')?;
    if !head.trim().eq_ignore_ascii_case("agent") {
        return None;
    }
    let token = tail
        .split_whitespace()
        .next()?
        .trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    agents
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(token))
        .map(|a| a.name.clone())
}

/// The election prompt: the purpose, the roster with each agent's own
/// advertised role, the exclusion (when judging), and the reply grammar.
fn prompt(purpose: &str, agents: &[AgentInfo], exclude: Option<&str>) -> String {
    let roster: Vec<String> = agents
        .iter()
        .map(|a| format!("{} \u{2014} {}", a.name, clip(&a.role, 120)))
        .collect();
    let not = exclude
        .map(|ex| format!(" (not {ex} \u{2014} it did the work in question)"))
        .unwrap_or_default();
    format!(
        "Pick the ONE agent best suited to {purpose}{not}.\n\
         Agents:\n{}\n\
         The FIRST line of your reply must be exactly `AGENT: <name>`.",
        roster.join("\n")
    )
}

#[cfg(test)]
#[path = "elect_tests.rs"]
mod tests;
