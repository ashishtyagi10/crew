//! Relative-time + metadata suffixes for message cards: turns a message's
//! epoch-ms `ts` and its plugin metadata (e.g. reply latency) into the muted
//! ` · 2m ago · 4.2s` tail of the card header, so the transcript doubles as a
//! readable log.
use std::time::{SystemTime, UNIX_EPOCH};

/// Unix-epoch milliseconds now (0 if the clock is before the epoch).
pub(crate) fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

/// A compact relative timestamp for epoch-ms `ts` at `now` — `now`, `42s ago`,
/// `5m ago`, `3h ago`, `2d ago`. `None` when `ts` isn't epoch milliseconds.
pub(crate) fn rel_time(ts: &str, now_ms: u64) -> Option<String> {
    let t: u64 = ts.parse().ok()?;
    let secs = now_ms.saturating_sub(t) / 1000;
    Some(match secs {
        0..=9 => "now".into(),
        10..=59 => format!("{secs}s ago"),
        60..=3_599 => format!("{}m ago", secs / 60),
        3_600..=86_399 => format!("{}h ago", secs / 3_600),
        _ => format!("{}d ago", secs / 86_400),
    })
}

/// The background-task id carried at the FRONT of a message's `meta`
/// (`"task:<id>"` or `"task:<id> \u{00b7} <latency>"`), if any.
pub(crate) fn task_tag(meta: &str) -> Option<u64> {
    let rest = meta.strip_prefix("task:")?;
    rest.split_whitespace().next()?.parse().ok()
}

/// `meta` with any leading `task:<id>` tag removed — i.e. just the latency the
/// header tail should show (`"task:3 \u{00b7} 0.0s"` -> `"0.0s"`;
/// `"task:3"` -> `""`; an untagged `"4.2s"` is returned unchanged).
pub(crate) fn strip_task_tag(meta: &str) -> &str {
    let Some(rest) = meta.strip_prefix("task:") else {
        return meta;
    };
    match rest.split_once(" \u{00b7} ") {
        Some((_id, latency)) => latency,
        None => "",
    }
}

/// A compact elapsed-duration string with three format buckets:
/// - <1s (0–999ms): `"0.Xs"` (e.g., `"0.9s"`);
/// - 1–99s (1000–99999ms): one decimal `"X.Xs"` (e.g., `"3.2s"`, `"61.0s"`);
/// - >99s (100000ms+): `"MmSSs"` with zero-padded seconds (e.g., `"10m05s"`, `"2m05s"`).
///   > This restores the spec's swarm-task-timings design for durations past 99s.
pub(crate) fn fmt_elapsed(ms: u64) -> String {
    if ms < 1_000 {
        // <1s: "0.Xs" format (truncate to one decimal, not round)
        let tenths = (ms / 100) as u32;
        format!("0.{tenths}s")
    } else if ms < 100_000 {
        // 1–99s: one decimal place
        format!("{:.1}s", ms as f64 / 1_000.0)
    } else {
        // >99s: "MmSSs" format with zero-padded seconds
        let total_secs = ms / 1_000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{mins}m{secs:02}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_time_buckets() {
        let now = 1_000_000_000;
        assert_eq!(rel_time("999995000", now).unwrap(), "now");
        assert_eq!(rel_time("999958000", now).unwrap(), "42s ago");
        assert_eq!(rel_time("999700000", now).unwrap(), "5m ago");
        assert_eq!(rel_time("989200000", now).unwrap(), "3h ago");
        assert_eq!(rel_time("827200000", now).unwrap(), "2d ago");
    }

    #[test]
    fn rel_time_rejects_non_numeric_ts() {
        assert_eq!(rel_time("", 1000), None);
        assert_eq!(rel_time("t", 1000), None);
    }

    #[test]
    fn task_tag_parses_the_leading_id() {
        assert_eq!(task_tag("task:3"), Some(3));
        assert_eq!(task_tag("task:3 \u{00b7} 0.0s"), Some(3));
        assert_eq!(task_tag(""), None);
        assert_eq!(task_tag("4.2s"), None);
        assert_eq!(task_tag("task:"), None);
        assert_eq!(task_tag("task:abc"), None);
    }

    #[test]
    fn strip_task_tag_keeps_only_the_latency() {
        assert_eq!(strip_task_tag("task:3 \u{00b7} 0.0s"), "0.0s");
        assert_eq!(strip_task_tag("task:3"), "");
        assert_eq!(strip_task_tag("4.2s"), "4.2s"); // untagged unchanged
        assert_eq!(strip_task_tag(""), "");
    }

    #[test]
    fn fmt_elapsed_edge_cases() {
        // <1s: "0.Xs" format (e.g., "0.9s")
        assert_eq!(fmt_elapsed(900), "0.9s");
        assert_eq!(fmt_elapsed(100), "0.1s");
        assert_eq!(fmt_elapsed(999), "0.9s");

        // 1-99s: one decimal place (e.g., "3.2s", "61.0s")
        assert_eq!(fmt_elapsed(3_200), "3.2s");
        assert_eq!(fmt_elapsed(61_000), "61.0s");
        assert_eq!(fmt_elapsed(99_900), "99.9s");

        // >99s: "MmSSs" format with zero-padded seconds
        // 605s = 10m05s; 125s = 2m05s
        assert_eq!(fmt_elapsed(125_000), "2m05s");
        assert_eq!(fmt_elapsed(605_000), "10m05s");
        assert_eq!(fmt_elapsed(3_661_000), "61m01s"); // 1h 1m 1s
    }
}
