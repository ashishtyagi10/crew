//! `next` — the one question people actually ask a resident: what happens next?
//!
//! `crew daemon watching` prints the whole list, which is the answer to a different question.
//! This is one line — the soonest standing intent — scriptable into a prompt or a status bar,
//! and readable from a phone as "what's next". A quiet clock is not an error: with nothing
//! standing it says so and exits 0.
use crate::ipc_cards::IntentCard;
use crate::ipc_types::{Reply, Request, PROTOCOL_V};

use super::intent::{until, Intent};

/// The sentence for an empty clock, on every face.
pub(crate) const NOTHING: &str = "crew is not watching for anything";

/// One line: the id, when, the cadence, the task.
fn row(id: &str, fire_ms: u64, repeat: &str, text: &str, now_ms: u64) -> String {
    format!(
        "{id}  {}  {repeat}  {}",
        until(fire_ms, now_ms),
        text.trim()
    )
}

/// The soonest of `intents`, as the CLI prints it.
pub(crate) fn line(intents: &[IntentCard], now_ms: u64) -> String {
    match intents.iter().min_by_key(|i| (i.fire_ms, &i.id)) {
        Some(i) => row(&i.id, i.fire_ms, &i.repeat, &i.text, now_ms),
        None => NOTHING.to_string(),
    }
}

/// The soonest of the live list, as a channel is answered.
pub(crate) fn soonest(intents: &[Intent], now_ms: u64) -> String {
    match intents.iter().min_by_key(|i| (i.fire_ms, &i.id)) {
        Some(i) => row(&i.id, i.fire_ms, &i.repeat.label(), &i.text, now_ms),
        None => NOTHING.to_string(),
    }
}

/// `crew daemon next`: the soonest standing intent, or the sentence — exit 0 either way.
pub(crate) fn next(inst: Option<&str>) -> i32 {
    match super::request(inst, &Request::Watching { v: PROTOCOL_V }) {
        Some(Reply::Watchlist { intents }) => {
            println!("{}", line(&intents, crate::chattime::unix_now_ms()));
            0
        }
        Some(other) => super::cli::unexpected(&other),
        None => super::cli::no_daemon(),
    }
}

#[cfg(test)]
#[path = "nextcli_tests.rs"]
mod tests;
