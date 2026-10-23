//! A day's end, or the week's, as a mail says it (`tododue`): "EOD" is that
//! day at 5pm, "EOD Friday" that Friday's, "end of the week" that Friday at
//! 5pm. Apart from `tododue`, which it kept over the line cap.
use chrono::{Datelike, Duration, NaiveDateTime};

use crate::todopane::duedate;

const EOD: [&str; 5] = [
    "end of the day",
    "end of day",
    "close of business",
    "eod",
    "cob",
];
const EOW: [&str; 3] = ["end of the week", "end of week", "eow"];

/// A day's end, or the week's, at the start of `phrase`. A day said after
/// the day's end is its day: "EOD Friday", "end of day tomorrow" were due
/// the day the mail was sent, overdue as soon as they were filed.
pub(super) fn end_of(phrase: &str, at: NaiveDateTime) -> Option<(NaiveDateTime, bool, String)> {
    let starts = |w: &str| {
        phrase.starts_with(w)
            && phrase[w.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric())
    };
    let five = |d: chrono::NaiveDate| d.and_hms_opt(17, 0, 0);
    if let Some(w) = EOD.iter().find(|w| starts(w)) {
        let rest = phrase[w.len()..].trim_start();
        let next = (rest
            .strip_prefix("on ")
            .unwrap_or(rest)
            .split_whitespace()
            .next())
        .unwrap_or("");
        let day = duedate::find(next, at)
            .filter(|h| h.start == 0 && h.end == next.chars().count() && !h.has_time);
        return Some(match day {
            Some(h) => (five(h.due.date())?, true, format!("{w} {next}")),
            None => (five(at.date())?, true, w.to_string()),
        });
    }
    if let Some(w) = EOW.iter().find(|w| starts(w)) {
        let ahead = (4 + 7 - at.weekday().num_days_from_monday()) % 7;
        return Some((
            five(at.date() + Duration::days(ahead.into()))?,
            true,
            w.to_string(),
        ));
    }
    // "next Friday [at 5pm]" is the grammar's (`nextday`), time and all.
    None
}
