//! `snooze <id> <for>` — the verb the clock was missing.
//!
//! A standing intent you cannot deal with at 7am could only be CANCELLED, and
//! cancelling loses it. Snoozing pushes the next firing by a duration the
//! cadence grammar already reads (`30m`, `2h`, `1d`) and leaves a repeat's
//! cadence where it was: the daily briefing snoozed half an hour is late
//! today and on time tomorrow. One appended entry, like a cancel, so it holds
//! whether or not the daemon is up.
use super::intent::Repeat;
use super::intentlog::{Entry, Watchlist};
use crate::ipc_types::{Reply, Request, PROTOCOL_V};

/// How long a snooze is, in ms, out of the word somebody typed. The cadence
/// grammar minus its cadences: `daily` is not an answer to "for how long".
pub(crate) fn delay_ms(word: &str) -> Option<u64> {
    if !word.trim().starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    match Repeat::parse(word)? {
        Repeat::Every { secs } => Some(secs * 1000),
        Repeat::Once => None,
    }
}

/// The sentence every face sends back when the delay does not parse.
pub(crate) const FOR_HOW_LONG: &str = "for how long? say 30m, 2h or 1d";

impl Watchlist {
    /// Push `id`'s next firing to `now_ms + delay_ms`. `Ok(None)` when nothing by that id is
    /// standing; `Ok(Some(fire_ms))` is where it landed.
    pub(crate) fn snooze(
        &self,
        id: &str,
        delay_ms: u64,
        now_ms: u64,
    ) -> std::io::Result<Option<u64>> {
        if !self.live().iter().any(|i| i.id == id) {
            return Ok(None);
        }
        let to_ms = now_ms.saturating_add(delay_ms);
        self.append(&Entry::Snoozed {
            id: id.to_string(),
            to_ms,
            at_ms: now_ms,
        })?;
        Ok(Some(to_ms))
    }
}

/// What a snooze did, as every face says it.
pub(crate) fn said(id: &str, landed: std::io::Result<Option<u64>>, now_ms: u64) -> String {
    match landed {
        Ok(Some(fire_ms)) => format!(
            "{id} snoozed \u{2014} {}",
            super::intent::until(fire_ms, now_ms)
        ),
        Ok(None) => format!("crew is not watching for {id}"),
        Err(e) => format!("could not write the watchlist: {e}"),
    }
}

/// `crew daemon snooze <id> <for>`: over the socket, so the daemon's own clock lands it.
pub(crate) fn cli(inst: Option<&str>, id: &str, word: &str) -> i32 {
    let Some(delay) = delay_ms(word) else {
        println!("{FOR_HOW_LONG}");
        return 2;
    };
    let req = Request::Snooze {
        v: PROTOCOL_V,
        id: id.to_string(),
        delay_ms: delay,
    };
    let now_ms = crate::chattime::unix_now_ms();
    match super::request(inst, &req) {
        Some(Reply::Snoozed { id, fire_ms }) => {
            println!("{}", said(&id, Ok(fire_ms), now_ms));
            i32::from(fire_ms.is_none())
        }
        Some(Reply::Failed { message }) => {
            println!("{message}");
            1
        }
        Some(other) => super::cli::unexpected(&other),
        None => super::cli::no_daemon(),
    }
}

#[cfg(test)]
#[path = "snooze_tests.rs"]
mod tests;
