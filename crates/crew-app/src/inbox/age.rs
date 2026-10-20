//! A row's age the way a mail client says it: `9m`, `3h`, `Mon`, `Oct 12`,
//! `2024`. Local time, so "Mon" is the user's Monday.
use chrono::{DateTime, Datelike, Local, TimeZone};

/// `at_ms` (ms since the epoch) told relative to `now_ms`.
pub(crate) fn said(at_ms: i64, now_ms: i64) -> String {
    // No date (Gmail left `internalDate` out) is no age, not 1970.
    let (Some(at), Some(now)) = (local(at_ms).filter(|_| at_ms > 0), local(now_ms)) else {
        return String::new();
    };
    let mins = (now_ms - at_ms).max(0) / 60_000;
    match mins {
        0 => "now".into(),
        m if m < 60 => format!("{m}m"),
        m if m < 24 * 60 => format!("{}h", m / 60),
        m if m < 6 * 24 * 60 => at.format("%a").to_string(),
        _ if at.year() == now.year() => at.format("%b %-d").to_string(),
        _ => at.year().to_string(),
    }
}

fn local(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

#[cfg(test)]
mod tests {
    const H: i64 = 3_600_000;
    const NOW: i64 = 1_792_500_000_000; // 2026-10-20

    #[test]
    fn ages_step_from_minutes_to_years() {
        assert_eq!(super::said(NOW - 30_000, NOW), "now");
        assert_eq!(super::said(NOW - 9 * 60_000, NOW), "9m");
        assert_eq!(super::said(NOW - 3 * H, NOW), "3h");
        assert_eq!(super::said(NOW - 3 * 24 * H, NOW).len(), 3, "a weekday");
        assert!(
            super::said(NOW - 40 * 24 * H, NOW).contains(' '),
            "a month and day"
        );
        assert_eq!(super::said(NOW - 800 * 24 * H, NOW), "2024");
        assert_eq!(
            super::said(NOW + H, NOW),
            "now",
            "a clock ahead is not the future"
        );
        assert_eq!(super::said(0, NOW), "", "no date, no age");
    }
}
