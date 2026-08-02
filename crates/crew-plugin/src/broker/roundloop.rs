//! The improvement loop: run the relay several times, each round handed the
//! previous round's answer to improve on. Reached through the intent router
//! (`/loop` is retired); `MAX_ROUNDS` is a BACKSTOP, not the driver — the
//! model ends a healthy run itself by leading a round's answer with `@done`.
use crate::PluginEvent;

use super::constructs::round_body;
use super::relay::{msg, relay_turn, split_target};
use super::session::Session;
use super::stdio::roster;

/// Hard ceiling on rounds, so a typo can't run a 100-round loop.
pub(crate) const MAX_ROUNDS: u32 = 10;

/// `loop <n> <task>`: run up to `n` relay rounds, feeding each round's answer
/// into the next as context to improve on.
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
        return emit(msg(
            "agent smith",
            format!("loop: give 1-{MAX_ROUNDS} rounds and a task"),
        ));
    };
    if task.is_empty() {
        return emit(msg(
            "agent smith",
            format!("loop: give 1-{MAX_ROUNDS} rounds and a task"),
        ));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    let (start, task) = split_target(task, &reg);
    let broker = session.broker(reg);
    let mut turn = |round: u32,
                    body: &str,
                    em: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>|
     -> anyhow::Result<Option<String>> {
        relay_turn(
            &broker,
            &start,
            body,
            &format!("loop-{round}"),
            tick_emit,
            em,
        )
    };
    rounds(session, n, &start, &task, emit, &mut turn)
}

/// One loop round behind the seam: `(round, body, emit)` → the round's
/// answer. Injected so a test can vary the answer per round, which a static
/// mock reply cannot.
pub(crate) type Turn<'a> = &'a mut dyn FnMut(
    u32,
    &str,
    &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<Option<String>>;

/// Drive up to `n` improvement rounds through `turn`. `n` is a BACKSTOP, not
/// the driver: the model ends the loop early by leading a round's answer
/// with `@done` (see [`early_done`]) once there is a result worth keeping.
pub(crate) fn rounds(
    session: &Session,
    n: u32,
    start: &str,
    task: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
    turn: Turn,
) -> anyhow::Result<()> {
    let mut answer: Option<String> = None;
    for round in 1..=n {
        if session.cancelled() {
            return emit(msg("agent smith", "loop cancelled by /stop"));
        }
        emit(msg(
            "agent smith",
            format!("loop round {round}/{n} \u{2014} starting with {start}"),
        ))?;
        // Once there is a result to keep, the agent is TOLD the early exit —
        // an exit the model cannot see is one it plans straight past.
        let mut body = round_body(task, answer.as_deref());
        if answer.is_some() {
            body.push_str(
                "\n\nIf the result above is already as good as it can get, \
                 reply with `@done` as the FIRST line, then the final version.",
            );
        }
        answer = turn(round, &body, emit)?.or(answer);
        if answer.as_deref().is_some_and(early_done) {
            return emit(msg(
                "agent smith",
                format!(
                    "loop done early after {round} round(s) \u{2014} the crew \
                     called it done"
                ),
            ));
        }
    }
    emit(msg(
        "agent smith",
        format!("loop done \u{2014} {n} round(s) complete"),
    ))
}

/// Whether a round's answer declares the loop finished: `@done` leading its
/// FIRST line — the same token the relay protocol's control line uses
/// (`route::parse_routing`), reused with the same tolerant trimming rather
/// than inventing a second grammar.
fn early_done(answer: &str) -> bool {
    answer
        .trim()
        .lines()
        .next()
        .map(|l| {
            l.trim()
                .trim_matches(|c: char| matches!(c, '*' | '`' | '_' | ' ' | '.'))
                .to_ascii_lowercase()
        })
        .is_some_and(|l| l.starts_with("@done"))
}

#[cfg(test)]
#[path = "constructs_round_tests.rs"]
mod tests;
