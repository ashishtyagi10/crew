//! Round-based constructs: `/loop <n> <task>` runs the relay `n` times, each
//! round handed the previous round's answer to improve on; `/goal` (see
//! `goal_cmd`) keeps looping until a judge agent says the goal is met.
use crate::PluginEvent;

use super::relay::{msg, relay_turn, split_target};
use super::route::clip;
use super::session::Session;
use super::stdio::{call_timeout, max_hops, roster, token_budget};
use super::Broker;

/// Hard ceiling on rounds, so a typo can't run a 100-round loop.
pub(crate) const MAX_ROUNDS: u32 = 10;

/// `/loop <n> <task>`: run `n` relay rounds, feeding each round's answer into
/// the next as context to improve on.
pub(crate) fn loop_cmd(
    session: &mut Session,
    rest: &str,
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
    let broker = Broker::new(reg, max_hops(), call_timeout()).with_budget(token_budget());
    let mut answer: Option<String> = None;
    for round in 1..=n {
        emit(msg(
            "crew",
            format!("loop round {round}/{n} \u{2014} starting with {start}"),
        ))?;
        let body = round_body(&task, answer.as_deref());
        let tid = format!("loop-{round}");
        answer = relay_turn(&broker, &start, &body, &tid, emit)?.or(answer);
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
    let judge = pick_judge(&reg.names(), &start);
    let timeout = call_timeout();
    let broker = Broker::new(reg, max_hops(), call_timeout()).with_budget(token_budget());
    let mut answer: Option<String> = None;
    for round in 1..=GOAL_ROUNDS {
        emit(msg(
            "crew",
            format!("goal round {round}/{GOAL_ROUNDS} \u{2014} {start} works, {judge} judges"),
        ))?;
        let body = round_body(&goal, answer.as_deref());
        answer = relay_turn(&broker, &start, &body, &format!("goal-{round}"), emit)?.or(answer);
        let Some(ans) = answer.as_deref() else {
            return emit(msg("crew", "goal stopped \u{2014} no answer was produced"));
        };
        emit(PluginEvent::Activity {
            agent: judge.clone(),
            state: "thinking".into(),
        })?;
        let verdict = broker
            .registry
            .get(&judge)
            .map(|a| a.call(&judge_prompt(&goal, ans), timeout));
        emit(PluginEvent::Activity {
            agent: String::new(),
            state: "idle".into(),
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

/// The judge: the reviewer when it isn't the worker, else any other agent,
/// else the worker itself (single-agent roster).
pub(crate) fn pick_judge(names: &[String], worker: &str) -> String {
    names
        .iter()
        .find(|n| n.as_str() == "reviewer" && n.as_str() != worker)
        .or_else(|| names.iter().find(|n| n.as_str() != worker))
        .cloned()
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
