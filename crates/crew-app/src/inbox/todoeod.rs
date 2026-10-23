//! A day's end, or the week's, as a mail says it (`tododue`): "EOD" is that
//! day at 5pm, "EOD Friday" and "the end of Friday" that Friday's, "end of
//! the week" that Friday at 5pm. Apart from `tododue`, which it kept over the line cap.
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
    // The longest run of words after it that is a day: "oct 30", "next friday".
    let day_in = |w: &str| {
        let rest = phrase[w.len()..].trim_start();
        let rest = rest.strip_prefix("on ").unwrap_or(rest);
        let ws: Vec<&str> = rest.split_whitespace().collect();
        (1..=ws.len()).rev().find_map(|n| {
            let s = ws[..n].join(" ");
            let h = duedate::find(&s, at)
                .filter(|h| h.start == 0 && h.end == s.chars().count() && !h.has_time)?;
            Some((five(h.due.date())?, true, format!("{w} {s}")))
        })
    };
    if let Some(w) = EOD.iter().find(|w| starts(w)) {
        return day_in(w).or_else(|| Some((five(at.date())?, true, w.to_string())));
    }
    if let Some(w) = EOW.iter().find(|w| starts(w)) {
        let ahead = (4 + 7 - at.weekday().num_days_from_monday()) % 7;
        return Some((
            five(at.date() + Duration::days(ahead.into()))?,
            true,
            w.to_string(),
        ));
    }
    // "the end of Friday" is Friday's; "the end of the month" no day's.
    if starts("end of") {
        return day_in("end of");
    }
    // "next Friday [at 5pm]" is the grammar's (`nextday`), time and all.
    None
}
