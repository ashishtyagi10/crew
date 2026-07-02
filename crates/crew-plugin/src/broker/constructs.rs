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
