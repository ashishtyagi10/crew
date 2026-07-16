//! Round-based constructs: `/loop <n> <task>` runs the relay `n` times, each
//! round handed the previous round's answer to improve on; `/goal` (see
//! `goal_cmd`) keeps looping until a judge agent says the goal is met.
use crate::{AgentInfo, PluginEvent};

use super::relay::{msg, relay_turn, split_target};
use super::route::clip;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// Hard ceiling on rounds, so a typo can't run a 100-round loop.
pub(crate) const MAX_ROUNDS: u32 = 10;

/// `/loop <n> <task>`: run `n` relay rounds, feeding each round's answer into
/// the next as context to improve on.
pub(crate) fn loop_cmd(
    session: &mut Session,
    rest: &str,
    tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let (n, task) = match rest.trim().split_once(char::is_whitespace) {
        Some((n, task)) => (n.parse::<u32>().ok(), task.trim()),
        None => (None, ""),
    };
    let Some(n) = n.filter(|n| (1..=MAX_ROUNDS).contains(n)) else {
        return emit(msg("crew", format!("usage: /loop <1-{MAX_ROUNDS}> <task>")));
    };
    if task.is_empty() {
        return emit(msg("crew", format!("usage: /loop <1-{MAX_ROUNDS}> <task>")));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", roster(&reg)));
    }
    let (start, task) = split_target(task, &reg);
    let broker = session.broker(reg);
    let mut answer: Option<String> = None;
    for round in 1..=n {
        if session.cancelled() {
            return emit(msg("crew", "loop cancelled by /stop"));
        }
        emit(msg(
            "crew",
            format!("loop round {round}/{n} \u{2014} starting with {start}"),
        ))?;
        let body = round_body(&task, answer.as_deref());
        let tid = format!("loop-{round}");
        answer = relay_turn(&broker, &start, &body, &tid, tick_emit, emit)?.or(answer);
    }
    emit(msg(
        "crew",
        format!("loop done \u{2014} {n} round(s) complete"),
    ))
}

/// Rounds `/goal` tries before giving up.
pub(crate) const GOAL_ROUNDS: u32 = 5;

/// `/goal <text>`: relay rounds until a judge agent rules the goal met, or the
/// round cap trips. The reviewer judges when present (someone other than the
/// worker), so the crew doesn't grade its own homework.
pub(crate) fn goal_cmd(
    session: &mut Session,
    rest: &str,
    tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let goal = rest.trim();
    if goal.is_empty() {
        return emit(msg("crew", "usage: /goal <what must be true when done>"));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", roster(&reg)));
    }
    let (start, goal) = split_target(goal, &reg);
    let judge = pick_judge(&reg.infos(), &start);
    let timeout = call_timeout();
    let broker = session.broker(reg);
    let mut answer: Option<String> = None;
    for round in 1..=GOAL_ROUNDS {
        if session.cancelled() {
            return emit(msg("crew", "goal cancelled by /stop"));
        }
        emit(msg(
            "crew",
            format!("goal round {round}/{GOAL_ROUNDS} \u{2014} {start} works, {judge} judges"),
        ))?;
        let body = round_body(&goal, answer.as_deref());
        answer = relay_turn(
            &broker,
            &start,
            &body,
            &format!("goal-{round}"),
            tick_emit,
            emit,
        )?
        .or(answer);
        let Some(ans) = answer.as_deref() else {
            return emit(msg("crew", "goal stopped \u{2014} no answer was produced"));
        };
        emit(PluginEvent::Activity {
            agent: judge.clone(),
            state: "thinking".into(),
            from: "goal".into(),
        })?;
        let verdict = broker
            .registry
            .get(&judge)
            .map(|a| a.call(&judge_prompt(&goal, ans), timeout));
        emit(PluginEvent::Activity {
            agent: String::new(),
            state: "idle".into(),
            from: String::new(),
        })?;
        let reply = match verdict {
            Some(Ok(r)) => r,
            Some(Err(e)) => {
                emit(msg("crew", format!("judge failed: {e} \u{2014} stopping")))?;
                return Ok(());
            }
            None => return emit(msg("crew", "goal stopped \u{2014} judge went missing")),
        };
        let (met, why) = parse_verdict(&reply);
        if met {
            return emit(msg(
                "crew",
                format!("goal met after {round} round(s) \u{2713} \u{2014} {why}"),
            ));
        }
        emit(msg(
            &format!("{judge} \u{2192} user"),
            format!("not met \u{2014} {why}"),
        ))?;
    }
    emit(msg(
        "crew",
        format!("goal not met after {GOAL_ROUNDS} rounds \u{2014} stopping (last answer above)"),
    ))
}

/// Whether an agent with this name/role advertises a review/critique
/// capability — the judge is chosen by the agent's OWN role (each specialist
/// is invented per task and carries its own hint; there is no static map to
/// look up any more), NOT by the literal name "reviewer", so a roster of
/// arbitrarily-named specialists still elects a critic. The literal name is
/// kept as a floor in case a custom agent's role is empty.
///
/// `pub(crate)` rather than duplicated: `/review` (`review.rs`) wants the
/// exact same critic election `/goal`'s judge does, and a second copy of this
/// keyword list would drift the moment one of them changed capability words.
pub(crate) fn is_critic(role: &str, name: &str) -> bool {
    role.contains("review") || role.contains("critique") || name == "reviewer"
}

/// Whether an agent with this name/role advertises a writing/build
/// capability — `/commit` and `/standup` (which draft prose *about* a diff or
/// a commit log, not code itself) want an author elected the same way
/// `is_critic` elects a judge: by the agent's OWN role, with the literal name
/// "coder" kept only as a floor for a custom agent whose role is empty.
pub(crate) fn is_writer(role: &str, name: &str) -> bool {
    role.contains("build")
        || role.contains("implement")
        || role.contains("cod") // "code", "coding", "coder"
        || role.contains("writ") // "write", "writing", "writer"
        || name == "coder"
}

/// Elect an agent from `agents` by capability: the first whose `(role, name)`
/// satisfies `is_match`, else the roster's first agent at all (mirroring
/// `split_target`'s own fallback), else empty — only reachable with an empty
/// roster, which every call site here has already ruled out via
/// `reg.is_empty()`. Shared by `/review`, `/commit` and `/standup` so each
/// elects by an agent's own advertised role rather than hoping a specialist
/// happens to be literally named "reviewer"/"coder" — no invented specialist
/// ever is (see `d49a6e1`, which deleted the inbuilt trio).
pub(crate) fn pick_by_role(agents: &[AgentInfo], is_match: impl Fn(&str, &str) -> bool) -> String {
    agents
        .iter()
        .find(|a| is_match(&a.role, &a.name))
        .or_else(|| agents.first())
        .map(|a| a.name.clone())
        .unwrap_or_default()
}

/// The judge: a capability critic that isn't the worker, else any other agent,
/// else the worker itself (single-agent roster). Reads each agent's own role
/// (`AgentInfo::role`, sourced from `Adapter::role()`) rather than a static
/// name-based lookup, so an invented specialist like `quality-auditor` is
/// elected on the strength of its own advertised capability.
pub(crate) fn pick_judge(agents: &[AgentInfo], worker: &str) -> String {
    agents
        .iter()
        .find(|a| a.name != worker && is_critic(&a.role, &a.name))
        .or_else(|| agents.iter().find(|a| a.name != worker))
        .map(|a| a.name.clone())
        .unwrap_or_else(|| worker.to_string())
}

fn judge_prompt(goal: &str, answer: &str) -> String {
    format!(
        "You are the judge. Goal: {goal}\n\nLatest result:\n{}\n\nIs the goal \
         fully met? Reply with exactly one line: `MET: <why>` or `NOT MET: \
         <what is missing>`.",
        clip(answer, 1500)
    )
}

/// Parse the judge's ruling: `(met, reason)`. Anything that doesn't clearly
/// say MET counts as not met — the conservative reading.
pub(crate) fn parse_verdict(reply: &str) -> (bool, String) {
    let clean = match crate::parse_routing(reply) {
        crate::Routing::Done(b) | crate::Routing::Relay { body: b, .. } if !b.is_empty() => b,
        _ => reply.trim().to_string(),
    };
    let first = clean.lines().next().unwrap_or("").trim();
    let upper = first.to_ascii_uppercase();
    let reason = first
        .split_once(':')
        .map(|(_, r)| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| clip(&clean, 200));
    (
        upper.starts_with("MET") && !upper.starts_with("NOT"),
        reason,
    )
}

/// The task for one round: the original task, plus the previous round's
/// answer (clipped) to refine when there is one.
pub(crate) fn round_body(task: &str, prev: Option<&str>) -> String {
    match prev {
        None => task.to_string(),
        Some(prev) => format!(
            "{task}\n\nPrevious round's result:\n{}\n\nImprove on it \u{2014} fix \
             weaknesses, keep what works.",
            clip(prev, 1500)
        ),
    }
}

#[cfg(test)]
#[path = "constructs_tests.rs"]
mod tests;
