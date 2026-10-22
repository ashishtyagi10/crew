//! The day a thread's newest mail came, as a band over the list (goal
//! 2026-10-20-inbox-list-look, item 8): `today`, `yesterday`, the weekday
//! for the rest of this week, `last week`, `earlier this month`, the month
//! for the rest of this year, then the year. Local time and weeks from
//! Monday, lowercase as `/todo`'s done history writes its days.
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};

/// The band for a thread whose newest mail came at `at_ms`.
pub(crate) fn band(at_ms: i64, now_ms: i64) -> String {
    let day = |ms: i64| {
        Local
            .timestamp_millis_opt(ms)
            .single()
            .map(|t| t.date_naive())
    };
    match (day(at_ms).filter(|_| at_ms > 0), day(now_ms)) {
        (Some(d), Some(today)) => label(d, today),
        _ => "undated".into(),
    }
}

pub(crate) fn label(d: NaiveDate, today: NaiveDate) -> String {
    let monday = |x: NaiveDate| x - Duration::days(i64::from(x.weekday().num_days_from_monday()));
    let (week, this_week) = (monday(d), monday(today));
    match (today - d).num_days() {
        // A clock ahead of ours is not the future.
        ..=0 => "today".into(),
        1 => "yesterday".into(),
        _ if week == this_week => d.format("%A").to_string().to_lowercase(),
        _ if week == this_week - Duration::days(7) => "last week".into(),
        _ if (d.year(), d.month()) == (today.year(), today.month()) => "earlier this month".into(),
        _ if d.year() == today.year() => d.format("%B").to_string().to_lowercase(),
        _ => d.year().to_string(),
    }
}

/// A `Date` header as local wall time (`reader::when` says why each form is tried).
pub(crate) fn sent(date: &str) -> Option<chrono::NaiveDateTime> {
    let d = date.trim();
    let bare = d.split(" (").next().unwrap_or(d);
    let dayless = bare.split_once(", ").map_or(bare, |(_, rest)| rest);
    [d, bare, dayless]
        .iter()
        .find_map(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
        .map(|t| t.with_timezone(&chrono::Local).naive_local())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    fn on(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }

    #[test]
    fn a_day_reads_nearest_first_then_by_week_month_and_year() {
        let today = on(10, 22); // a Thursday
        let said = |d| super::label(d, today);
        assert_eq!(said(on(10, 23)), "today", "a clock ahead");
        assert_eq!(said(on(10, 22)), "today");
        assert_eq!(said(on(10, 21)), "yesterday");
        assert_eq!(said(on(10, 19)), "monday");
        assert_eq!(said(on(10, 18)), "last week", "the Sunday before");
        assert_eq!(said(on(10, 12)), "last week");
        assert_eq!(said(on(10, 11)), "earlier this month");
        assert_eq!(said(on(9, 30)), "september");
        assert_eq!(said(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()), "2025");
        let monday = super::label(on(10, 18), on(10, 19));
        assert_eq!(monday, "yesterday", "yesterday wins over last week");
        assert_eq!(super::band(0, 1_792_500_000_000), "undated");
    }
}
