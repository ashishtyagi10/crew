//! `crew daemon at | watching | cancel | snooze` — the clock's CLI face.
//!
//! The time is read OUT OF THE TASK, the way the todo composer reads it out of a title
//! (`crate::todopane::duedate`): `crew daemon at "tomorrow 9am brief me on the calendar"` sets
//! the alarm for 9am and gives the agent "brief me on the calendar". One grammar for both
//! surfaces means "in 2 weeks", "friday 5pm" and "tomorrow" already work here, and a phrase that
//! parses in one place cannot mysteriously fail in the other.
//!
//! The parsing is done HERE rather than in the daemon on purpose: the user's clock and timezone
//! are on this side of the socket, and the wire carries an absolute epoch millisecond.
use crate::ipc_types::{Reply, Request, PROTOCOL_V};

use super::cli::flag;

use super::intent::{until, Repeat};

/// Every non-flag argument after `after`, joined back into the one sentence the user typed.
/// `crew daemon at tomorrow 9am the forecast` reaches us as five words and is one phrase.
fn positionals(args: &[String], after: usize) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut it = args.iter().skip(after);
    while let Some(a) = it.next() {
        if a.starts_with("--") {
            it.next();
            continue;
        }
        out.push(a);
    }
    out.join(" ")
}

/// `crew daemon at | watching | cancel`, dispatched from [`super::cli::run_sub`].
pub(super) fn sub(inst: Option<&str>, args: &[String], usage: &str) -> i32 {
    match args.first().map(String::as_str) {
        Some("watching") => watching(inst),
        Some("cancel") => match args.get(1) {
            Some(id) => cancel(inst, id),
            None => {
                print!("{usage}");
                2
            }
        },
        Some("snooze") => match (args.get(1), args.get(2)) {
            (Some(id), Some(word)) => super::snooze::cli(inst, id, word),
            _ => {
                print!("{usage}");
                2
            }
        },
        _ => {
            let said = positionals(args, 1);
            if said.trim().is_empty() {
                print!("{usage}");
                return 2;
            }
            at(inst, &said, flag(args, "--to"), flag(args, "--every"))
        }
    }
}

/// What `crew daemon at` needs before it can register anything.
#[derive(Debug)]
pub(crate) struct Parsed {
    pub text: String,
    pub fire_ms: u64,
    pub repeat_secs: Option<u64>,
}

/// Read a standing intent out of what somebody typed. `Err` carries the sentence to print —
/// every failure here is a person's phrasing, so it says what would have worked.
pub(crate) fn parse(said: &str, every: Option<&str>, now_ms: u64) -> Result<Parsed, String> {
    let repeat_secs = match every {
        None => None,
        Some(word) => match Repeat::parse(word) {
            Some(Repeat::Every { secs }) => Some(secs),
            Some(Repeat::Once) => None,
            None => {
                return Err(format!(
                    "I do not know the cadence {word:?} \u{2014} try daily, weekly, hourly, \
                     or every 30m"
                ))
            }
        },
    };
    let now = crate::todopane::duedate::now_local();
    let Some(hit) = crate::todopane::duedate::find(said, now) else {
        return Err(
            "I could not find a time in that \u{2014} try \"tomorrow 9am the forecast\"".into(),
        );
    };
    let text = crate::todopane::duedate::strip(said, hit.start, hit.end);
    if text.trim().is_empty() {
        return Err("that is a time with nothing to do at it".into());
    }
    let Some(fire_ms) = crate::todopane::duedate::to_epoch_ms(hit.due) else {
        return Err("that time is outside the range crew can hold".into());
    };
    // A repeat whose first firing is in the past would fire immediately and then again on its
    // cadence, which is never what "every day at 9" meant when it is said at 10.
    if fire_ms <= now_ms {
        return Err(format!(
            "that time has already passed \u{2014} {} ago. Say when it should NEXT happen.",
            super::intent::spell((now_ms - fire_ms) / 1000)
        ));
    }
    Ok(Parsed {
        text,
        fire_ms,
        repeat_secs,
    })
}

/// `crew daemon at <text>`: register one.
pub(crate) fn at(inst: Option<&str>, said: &str, to: Option<&str>, every: Option<&str>) -> i32 {
    let now_ms = crate::chattime::unix_now_ms();
    let parsed = match parse(said, every, now_ms) {
        Ok(p) => p,
        Err(e) => {
            println!("{e}");
            return 2;
        }
    };
    let req = Request::Watch {
        v: PROTOCOL_V,
        text: parsed.text.clone(),
        to: to.unwrap_or_default().to_string(),
        fire_ms: parsed.fire_ms,
        repeat_secs: parsed.repeat_secs,
    };
    match super::request(inst, &req) {
        Some(Reply::Watched { id, fire_ms }) => {
            println!("{id}  {}  {}", until(fire_ms, now_ms), parsed.text.trim());
            0
        }
        Some(Reply::Failed { message }) => {
            println!("{message}");
            1
        }
        Some(other) => super::cli::unexpected(&other),
        None => super::cli::no_daemon(),
    }
}

/// `crew daemon watching`: what is standing, soonest first.
pub(crate) fn watching(inst: Option<&str>) -> i32 {
    match super::request(inst, &Request::Watching { v: PROTOCOL_V }) {
        Some(Reply::Watchlist { intents }) => {
            if intents.is_empty() {
                println!("crew is not watching for anything");
            }
            let now_ms = crate::chattime::unix_now_ms();
            for i in intents {
                println!(
                    "{}  {}  {}  {}  {}",
                    i.id,
                    until(i.fire_ms, now_ms),
                    i.repeat,
                    i.to,
                    i.text
                );
            }
            0
        }
        Some(other) => super::cli::unexpected(&other),
        None => super::cli::no_daemon(),
    }
}

/// `crew daemon cancel <id>`: call one off.
pub(crate) fn cancel(inst: Option<&str>, id: &str) -> i32 {
    let req = Request::Unwatch {
        v: PROTOCOL_V,
        id: id.to_string(),
    };
    match super::request(inst, &req) {
        Some(Reply::Unwatched { id, found: true }) => {
            println!("{id} cancelled");
            0
        }
        Some(Reply::Unwatched { id, found: false }) => {
            println!("crew is not watching for {id}");
            1
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
#[path = "watchcli_tests.rs"]
mod tests;
