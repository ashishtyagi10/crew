//! Natural-language due dates typed straight into the todo composer:
//! `pay rent tomorrow`, `ship build fri 5pm`, `dentist aug 15`, `in 2 weeks`.
//!
//! [`find`] scans the input for the rightmost (then longest) token window
//! that parses under a small, explicit grammar — no crate, no heuristics
//! beyond it — and returns its char range so the composer can tint the
//! fragment live and [`strip`] can remove exactly what was highlighted on
//! save. Unrecognised words are never eaten: a bare month name (`may`),
//! bare digits (`pay 5 bills`) and `@tokens` don't parse.
//!
//! The grammar: `today` · `tomorrow` · weekday names (next occurrence,
//! today included) · `in N days|weeks` · `MMM D` / `D MMM` (next
//! occurrence) · `YYYY-MM-DD` · a time `5pm` / `5:30pm` / `17:30` alone
//! (today, else tomorrow once passed) or beside any date form.
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};

/// Hour a date-only due lands on (a morning reminder); its label hides the
/// clock (`due_has_time == false`).
pub(crate) const DEFAULT_HOUR: u32 = 9;

/// Widest token window the grammar can need: `in 2 weeks 5pm`.
const MAX_WINDOW: usize = 4;

/// A recognised date fragment: its char range in the input and what it
/// resolves to.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DueHit {
    pub start: usize,
    pub end: usize,
    pub due: NaiveDateTime,
    pub has_time: bool,
}

/// The rightmost-then-longest parsing token window of `input`, if any.
pub(crate) fn find(input: &str, now: NaiveDateTime) -> Option<DueHit> {
    let toks = tokens(input);
    let mut best: Option<((usize, usize), DueHit)> = None;
    for s in 0..toks.len() {
        for len in 1..=MAX_WINDOW.min(toks.len() - s) {
            let window = &toks[s..s + len];
            // `@project` tokens never take part in a date window (and a
            // window may not straddle one — stripping would eat the tag).
            if window.iter().any(|t| t.2.starts_with('@')) {
                continue;
            }
            let words: Vec<&str> = window.iter().map(|t| t.2.as_str()).collect();
            let Some((due, has_time)) = parse_window(&words, now) else {
                continue;
            };
            let key = (s + len, len); // rightmost end wins, then longer
            if best.as_ref().is_none_or(|(k, _)| key >= *k) {
                best = Some((
                    key,
                    DueHit {
                        start: window[0].0,
                        end: window[len - 1].1,
                        due,
                        has_time,
                    },
                ));
            }
        }
    }
    best.map(|(_, h)| h)
}

/// `input` with chars `[start, end)` removed and whitespace normalised —
/// exactly the highlighted fragment leaves the title.
pub(crate) fn strip(input: &str, start: usize, end: usize) -> String {
    let rest: String = input
        .chars()
        .take(start)
        .chain(input.chars().skip(end))
        .collect();
    rest.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whitespace-delimited tokens as (start_char, end_char, lowercased word).
fn tokens(input: &str) -> Vec<(usize, usize, String)> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect::<String>().to_lowercase();
        out.push((start, i, word));
    }
    out
}

/// One token window against the whole grammar.
fn parse_window(w: &[&str], now: NaiveDateTime) -> Option<(NaiveDateTime, bool)> {
    if let Some(d) = parse_date(w, now.date()) {
        return Some((d.and_hms_opt(DEFAULT_HOUR, 0, 0)?, false));
    }
    if w.len() >= 2 {
        if let (Some(t), Some(d)) = (
            parse_time(w[w.len() - 1]),
            parse_date(&w[..w.len() - 1], now.date()),
        ) {
            return Some((d.and_time(t), true));
        }
        if let (Some(t), Some(d)) = (parse_time(w[0]), parse_date(&w[1..], now.date())) {
            return Some((d.and_time(t), true));
        }
    }
    if w.len() == 1 {
        if let Some(t) = parse_time(w[0]) {
            // A bare time means today — or tomorrow once it has passed.
            let d = if t > now.time() {
                now.date()
            } else {
                now.date().succ_opt()?
            };
            return Some((d.and_time(t), true));
        }
    }
    None
}

/// The date half of the grammar (no time), against `today`.
fn parse_date(w: &[&str], today: NaiveDate) -> Option<NaiveDate> {
    match w {
        [one] => {
            if *one == "today" {
                return Some(today);
            }
            if *one == "tomorrow" {
                return today.succ_opt();
            }
            if let Some(wd) = weekday(one) {
                let ahead = (wd + 7 - today.weekday().num_days_from_monday()) % 7;
                return today.checked_add_signed(Duration::days(ahead as i64));
            }
            NaiveDate::parse_from_str(one, "%Y-%m-%d").ok()
        }
        ["in", n, unit] => {
            let n: i64 = n.parse().ok().filter(|&n| (1..=3650).contains(&n))?;
            let days = match *unit {
                "day" | "days" => n,
                "week" | "weeks" => n * 7,
                _ => return None,
            };
            today.checked_add_signed(Duration::days(days))
        }
        [a, b] => {
            let (m, d) = month(a)
                .zip(day_num(b))
                .or_else(|| month(b).zip(day_num(a)))?;
            // Next occurrence: this year while it hasn't passed, else next.
            match NaiveDate::from_ymd_opt(today.year(), m, d) {
                Some(t) if t >= today => Some(t),
                _ => NaiveDate::from_ymd_opt(today.year() + 1, m, d),
            }
        }
        _ => None,
    }
}

/// Weekday token → days-from-Monday index. Short and long forms.
fn weekday(w: &str) -> Option<u32> {
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

/// Month-name token → 1-based month. Bare month names only parse beside a
/// day number (see `parse_date`), so `may` alone stays title text.
fn month(w: &str) -> Option<u32> {
    Some(match w {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    })
}

/// Day-of-month token: digits with an optional ordinal suffix (`15th`).
fn day_num(w: &str) -> Option<u32> {
    let w = w
        .strip_suffix("st")
        .or_else(|| w.strip_suffix("nd"))
        .or_else(|| w.strip_suffix("rd"))
        .or_else(|| w.strip_suffix("th"))
        .unwrap_or(w);
    w.parse().ok().filter(|&d| (1..=31).contains(&d))
}

/// Time token: `5pm`, `5:30pm`, `12am`, `17:30`. Bare digits without a
/// colon or am/pm are NOT a time — `pay 5 bills` must not parse.
fn parse_time(w: &str) -> Option<NaiveTime> {
    let (body, pm) = if let Some(b) = w.strip_suffix("pm") {
        (b, Some(true))
    } else if let Some(b) = w.strip_suffix("am") {
        (b, Some(false))
    } else {
        (w, None)
    };
    if body.is_empty() || (pm.is_none() && !body.contains(':')) {
        return None;
    }
    let (h, m): (u32, u32) = match body.split_once(':') {
        Some((h, m)) if m.len() == 2 => (h.parse().ok()?, m.parse().ok().filter(|&m| m < 60)?),
        Some(_) => return None,
        None => (body.parse().ok()?, 0),
    };
    match pm {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            let h24 = match (h, pm) {
                (12, false) => 0,
                (12, true) => 12,
                (h, true) => h + 12,
                (h, false) => h,
            };
            NaiveTime::from_hms_opt(h24, m, 0)
        }
        None => NaiveTime::from_hms_opt(h, m, 0),
    }
}

// --- clock/format helpers -------------------------------------------------

/// Local wall-clock now, naive — the parser's reference point.
pub(crate) fn now_local() -> NaiveDateTime {
    Local::now().naive_local()
}

/// Local naive datetime → epoch ms. `earliest()` picks the pre-transition
/// instant on a DST-ambiguous wall time.
pub(crate) fn to_epoch_ms(due: NaiveDateTime) -> Option<u64> {
    let ms = Local
        .from_local_datetime(&due)
        .earliest()?
        .timestamp_millis();
    u64::try_from(ms).ok()
}

/// Epoch ms → local naive datetime.
/// `due` shifted by `days` whole CALENDAR days, wall clock kept — a 9:00
/// due stays 9:00 across a DST boundary (instant math would drift it an
/// hour). Any unrepresentable local time falls back to the input unshifted.
pub(crate) fn shift_days(due_ms: u64, days: i64) -> u64 {
    let Some(naive) = from_epoch_ms(due_ms) else {
        return due_ms;
    };
    let Some(date) = naive.date().checked_add_signed(Duration::days(days)) else {
        return due_ms;
    };
    to_epoch_ms(NaiveDateTime::new(date, naive.time())).unwrap_or(due_ms)
}

pub(crate) fn from_epoch_ms(ms: u64) -> Option<NaiveDateTime> {
    Some(
        Local
            .timestamp_millis_opt(ms as i64)
            .earliest()?
            .naive_local(),
    )
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// Humane row label for a due instant: `today` / `tomorrow` / `yesterday`,
/// a weekday within the next six days, else `aug 15` — plus `HH:MM` when
/// the user typed an explicit time.
pub(crate) fn label(due_ms: u64, has_time: bool, now_ms: u64) -> String {
    match (from_epoch_ms(due_ms), from_epoch_ms(now_ms)) {
        (Some(d), Some(n)) => label_naive(d, has_time, n),
        _ => String::new(),
    }
}

/// [`label`] on naive datetimes — the testable half.
pub(crate) fn label_naive(due: NaiveDateTime, has_time: bool, now: NaiveDateTime) -> String {
    let dd = (due.date() - now.date()).num_days();
    let date = match dd {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        -1 => "yesterday".to_string(),
        2..=6 => DAYS[due.date().weekday().num_days_from_monday() as usize].to_string(),
        _ => format!(
            "{} {}",
            MONTHS[due.date().month0() as usize],
            due.date().day()
        ),
    };
    if has_time {
        format!("{date} {:02}:{:02}", due.time().hour(), due.time().minute())
    } else {
        date
    }
}

/// Signed days between a due instant and now (negative = overdue by days),
/// on calendar dates. `None` when either stamp doesn't convert.
pub(crate) fn days_from_now(due_ms: u64, now_ms: u64) -> Option<i64> {
    let (d, n) = (from_epoch_ms(due_ms)?, from_epoch_ms(now_ms)?);
    Some((d.date() - n.date()).num_days())
}

/// Round-trippable text for re-editing an item's due in the composer:
/// `2026-08-15` (+ ` 17:00` with an explicit time). Appended at the END of
/// the edit line, so rightmost-wins re-parsing can't grab a date-looking
/// word inside the title instead.
pub(crate) fn edit_text(due_ms: u64, has_time: bool) -> Option<String> {
    let d = from_epoch_ms(due_ms)?;
    Some(if has_time {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            d.year(),
            d.month(),
            d.day(),
            d.time().hour(),
            d.time().minute()
        )
    } else {
        format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
    })
}

#[cfg(test)]
#[path = "duedate_tests.rs"]
mod tests;
