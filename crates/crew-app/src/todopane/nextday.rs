//! `next friday` and `this friday` in the `/todo` composer, the commonest
//! ways to name a day. The grammar had neither: only the weekday parsed,
//! `next` stayed in the title, and "call bob next friday" filed "call bob
//! next", due this Friday. The inbox already read "next friday" (its
//! `tododue::end_of`), and the todo card refused the same words as no date.

use chrono::{Datelike, Duration, NaiveDate};

/// Weekday token → days-from-Monday index. Short and long forms.
pub(super) fn weekday(w: &str) -> Option<u32> {
    Some(match w {
        "mon" | "monday" => 0,
        "tue" | "tues" | "tuesday" => 1,
        "wed" | "weds" | "wednesday" => 2,
        "thu" | "thur" | "thurs" | "thursday" => 3,
        "fri" | "friday" => 4,
        "sat" | "saturday" => 5,
        "sun" | "sunday" => 6,
        _ => return None,
    })
}

/// `this WD` is the coming one, today included, as a bare weekday is. `next
/// WD` is the one of the week after when the coming one falls in this week
/// (said on a Tuesday, next Friday is not this Friday) and the coming one
/// otherwise: the inbox's reading, so the card and the composer agree.
pub(super) fn date(which: &str, wd: &str, today: NaiveDate) -> Option<NaiveDate> {
    let now = today.weekday().num_days_from_monday();
    let ahead = (weekday(wd)? + 7 - now) % 7;
    let this_week = ahead < 7 - now;
    let more = if which == "next" && this_week { 7 } else { 0 };
    today.checked_add_signed(Duration::days(i64::from(ahead + more)))
}

#[cfg(test)]
#[path = "nextday_tests.rs"]
mod tests;
