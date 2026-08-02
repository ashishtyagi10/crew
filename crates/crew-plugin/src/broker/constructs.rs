//! `/goal` — relay rounds until a judge agent (elected by the model, see
//! `elect`) rules the goal met, or the round cap trips. The improvement
//! loop's machinery lives in `roundloop`; `round_body` is shared from here.
use crate::PluginEvent;

use super::relay::{msg, relay_turn, split_target};
use super::route::clip;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// Rounds `/goal` tries before giving up.
pub(crate) const GOAL_ROUNDS: u32 = 5;

/// `/goal <text>`: relay rounds until a judge agent rules the goal met, or the
/// round cap trips. The MODEL elects the judge from the roster (someone other
/// than the worker, so the crew doesn't grade its own homework); keyless and
/// mock runs fall back to the first non-worker deterministically.
pub(crate) fn goal_cmd(
    session: &mut Session,
    rest: &str,
    tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    match crate::broker::intent::live_classifier() {
        Some(c) => goal_cmd_with(session, rest, tick_emit, emit, Some(&c)),
        None => goal_cmd_with(session, rest, tick_emit, emit, None),
    }
}

/// [`goal_cmd`] with the judge-election call injected — the test seam,
/// mirroring `intent::route_with`.
pub(crate) fn goal_cmd_with(
    session: &mut Session,
    rest: &str,
    tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
    elector: Option<crate::broker::intent::Classifier>,
) -> anyhow::Result<()> {
    let goal = rest.trim();
    if goal.is_empty() {
        return emit(msg(
            "agent smith",
            "usage: /goal <what must be true when done>",
        ));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    let (start, goal) = split_target(goal, &reg);
    let judge = super::elect::elect_with(
        &format!("judge whether this goal is met: {}", clip(&goal, 200)),
        &reg.infos(),
        Some(&start),
        elector,
    );
    let timeout = call_timeout();
    let broker = session.broker(reg);
    let mut answer: Option<String> = None;
    for round in 1..=GOAL_ROUNDS {
        if session.cancelled() {
            return emit(msg("agent smith", "goal cancelled by /stop"));
        }
        emit(msg(
            "agent smith",
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
            return emit(msg(
                "agent smith",
                "goal stopped \u{2014} no answer was produced",
            ));
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
                emit(msg(
                    "agent smith",
                    format!("judge failed: {e} \u{2014} stopping"),
                ))?;
                return Ok(());
            }
            None => {
                return emit(msg(
                    "agent smith",
                    "goal stopped \u{2014} judge went missing",
                ))
            }
        };
        let (met, why) = parse_verdict(&reply);
        if met {
            return emit(msg(
                "agent smith",
                format!("goal met after {round} round(s) \u{2713} \u{2014} {why}"),
            ));
        }
        emit(msg(
            &format!("{judge} \u{2192} user"),
            format!("not met \u{2014} {why}"),
        ))?;
    }
    emit(msg(
        "agent smith",
        format!("goal not met after {GOAL_ROUNDS} rounds \u{2014} stopping (last answer above)"),
    ))
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
