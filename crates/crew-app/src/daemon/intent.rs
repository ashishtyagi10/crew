//! Standing intents — the clock (goal: docs/superpowers/goals/2026-09-01-close-the-open-goals.md,
//! Pillar 1). Crew could hold a conversation and could not hold an appointment: the daemon routes
//! a message from a channel to an agent and back, and nothing in it fired on time.
//!
//! An intent is one record — what to run, when it fires, where the answer goes, whether it comes
//! back — and this module is the record and its arithmetic only. Persistence is
//! [`super::intentlog`]; the firing is the daemon's loop.
//!
//! **Two rules live here rather than at the call site**, because both are about honesty and a
//! caller may not think to ask. A repeat that was missed while the machine slept ROLLS FORWARD to
//! the next occurrence and says how many it stepped over — running four skipped alarms in a burst
//! at breakfast is worse than saying it missed them. And a firing late by more than [`GRACE_MS`]
//! carries a note saying so, because "your 7am briefing" arriving at 11am with no comment is a
//! lie about when crew looked.
use serde::{Deserialize, Serialize};

/// How long after its time a firing may still be delivered without comment. Under this the delay
/// is the poll interval and a note would be noise; over it, something was asleep.
pub(crate) const GRACE_MS: u64 = 5 * 60 * 1000;

/// How often an intent comes back.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "every")]
pub(crate) enum Repeat {
    /// Fires once and is done.
    Once,
    /// Fires every `secs` seconds, for as long as it is not cancelled.
    Every { secs: u64 },
}

/// One day and one week, the two repeats anybody asks for by name.
pub(crate) const DAY_SECS: u64 = 24 * 60 * 60;
pub(crate) const WEEK_SECS: u64 = 7 * DAY_SECS;

impl Repeat {
    /// A repeat spelled the way somebody would type it: `daily`, `every day`, `hourly`,
    /// `weekly`, `every 30m`, `every 2h`. `None` for anything else — an unrecognised cadence
    /// must not quietly become "once", which is a different promise.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let t = text.trim().to_lowercase();
        let t = t.strip_prefix("every ").unwrap_or(&t).trim();
        Some(match t {
            "once" => Repeat::Once,
            "hourly" | "hour" => Repeat::Every { secs: 3_600 },
            "daily" | "day" => Repeat::Every { secs: DAY_SECS },
            "weekly" | "week" => Repeat::Every { secs: WEEK_SECS },
            _ => Repeat::Every {
                secs: every_secs(t)?,
            },
        })
    }

    /// How it reads back: the word somebody would have typed.
    pub(crate) fn label(&self) -> String {
        let secs = match self {
            Repeat::Once => return "once".to_string(),
            Repeat::Every { secs } => *secs,
        };
        match secs {
            3_600 => "hourly".to_string(),
            DAY_SECS => "daily".to_string(),
            WEEK_SECS => "weekly".to_string(),
            s if s % DAY_SECS == 0 => format!("every {}d", s / DAY_SECS),
            s if s % 3_600 == 0 => format!("every {}h", s / 3_600),
            s if s % 60 == 0 => format!("every {}m", s / 60),
            s => format!("every {s}s"),
        }
    }
}

/// `30m` / `2h` / `3d` / `90s` as seconds. Zero is rejected: an every-nothing repeat is a busy
/// loop wearing a schedule.
fn every_secs(t: &str) -> Option<u64> {
    let (digits, unit) = t.split_at(t.find(|c: char| !c.is_ascii_digit())?);
    let n: u64 = digits.parse().ok()?;
    let mult = match unit.trim() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3_600,
        "d" | "day" | "days" => DAY_SECS,
        "w" | "week" | "weeks" => WEEK_SECS,
        _ => return None,
    };
    (n > 0).then_some(n * mult)
}

/// One thing crew is watching for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Intent {
    /// Short, stable, and what a person types to cancel it: `w1`, `w2`.
    pub id: String,
    /// The task, verbatim, as it will be handed to an agent.
    pub text: String,
    /// Where the answer goes — a channel address (`telegram:4242`), the way a message that
    /// arrived over one is answered.
    pub to: String,
    /// Epoch ms of the next firing.
    pub fire_ms: u64,
    pub repeat: Repeat,
    /// Epoch ms it was registered, so a listing can say how long it has been standing.
    pub created_ms: u64,
}

impl Intent {
    /// Has its time come?
    pub(crate) fn due(&self, now_ms: u64) -> bool {
        self.fire_ms <= now_ms
    }

    /// Where this intent stands after firing at `now_ms`: the next firing, and how many whole
    /// occurrences were stepped over to get there. `None` ends it — a one-shot has no next.
    ///
    /// The skipping is the point. A daily intent on a laptop shut for a week comes back to seven
    /// past-due firings; running them all is six wrong answers and one right one.
    pub(crate) fn advance(&self, now_ms: u64) -> Option<Rolled> {
        let Repeat::Every { secs } = self.repeat else {
            return None;
        };
        let step = secs.max(1) * 1000;
        let mut next = self.fire_ms.saturating_add(step);
        let mut skipped = 0;
        while next <= now_ms {
            next = next.saturating_add(step);
            skipped += 1;
        }
        Some(Rolled { next, skipped })
    }

    /// The note a late firing carries, or `None` when it is on time. Late means later than
    /// [`GRACE_MS`] — below that the lag is the daemon's poll and saying so would be noise.
    pub(crate) fn late_note(&self, now_ms: u64) -> Option<String> {
        let late = now_ms.checked_sub(self.fire_ms)?;
        (late > GRACE_MS).then(|| {
            format!(
                "(this was due {} ago \u{2014} crew was not running then)",
                spell(late / 1000)
            )
        })
    }
}

/// Where a repeating intent landed after a firing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rolled {
    /// Epoch ms of the next firing, always in the future of the firing that produced it.
    pub next: u64,
    /// Occurrences that fell in the gap and will not be run.
    pub skipped: u64,
}

/// A duration in the coarsest unit that still says something: `45s`, `20m`, `4h`, `3d`.
pub(crate) fn spell(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3_599 => format!("{}m", secs / 60),
        3_600..=86_399 => format!("{}h", secs / 3_600),
        _ => format!("{}d", secs / 86_400),
    }
}

/// How long until `fire_ms`, spelled for a listing: `in 20m`, `in 3d`, and `now` for anything
/// already due — a past-due row must not read as a future one.
pub(crate) fn until(fire_ms: u64, now_ms: u64) -> String {
    match fire_ms.checked_sub(now_ms) {
        Some(ms) if ms >= 1000 => format!("in {}", spell(ms / 1000)),
        _ => "now".to_string(),
    }
}

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
