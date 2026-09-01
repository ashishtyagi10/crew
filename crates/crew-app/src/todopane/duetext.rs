//! How a due date READS and how it converts: `tomorrow`, `fri`, `aug 15`, the
//! text an edit puts back in the composer, and the epoch-millisecond
//! arithmetic underneath.
//!
//! Split from [`super::duedate`] for the line cap, along the line between
//! PARSING what someone typed and saying a date back to them.
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike};

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

pub(crate) const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

pub(crate) const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

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

/// Day-header label for the done-history view: `today` / `yesterday`, else
/// `aug 10` — with the year appended when it isn't this year.
pub(crate) fn day_label_naive(d: NaiveDate, today: NaiveDate) -> String {
    match (today - d).num_days() {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        _ if d.year() == today.year() => format!("{} {}", MONTHS[d.month0() as usize], d.day()),
        _ => format!("{} {} {}", MONTHS[d.month0() as usize], d.day(), d.year()),
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
