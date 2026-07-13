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

/// A compact elapsed-duration string — one decimal place at any magnitude
/// (`"0.9s"`, `"3.2s"`, `"61.0s"`, `"125.0s"`), matching the existing
/// turn-duration style in `chathdr::status_segments`
/// (`format!(" \u{00b7} {:.1}s", turn_ms as f64 / 1_000.0)`) rather than the
/// `MmSSs` bucket the swarm-task-timings design sketches for durations past
/// 99s — see the design doc's "Duration formatter" note for the deviation:
/// consistency with the header's existing style wins over a third format.
pub(crate) fn fmt_elapsed(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1_000.0)
}

/// The card header's metadata tail: ` · <rel time>` then ` · <meta>`, each part
/// present only when known. Empty when there's nothing to say.
pub(crate) fn meta_suffix(ts: &str, meta: &str, now_ms: u64) -> String {
    let meta = strip_task_tag(meta);
    let mut s = String::new();
    if let Some(rel) = rel_time(ts, now_ms) {
        s.push_str(" \u{00b7} ");
        s.push_str(&rel);
    }
    if !meta.is_empty() {
        s.push_str(" \u{00b7} ");
        s.push_str(meta);
    }
    s
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
    fn meta_suffix_combines_time_and_latency() {
        assert_eq!(
            meta_suffix("999958000", "4.2s", 1_000_000_000),
            " \u{00b7} 42s ago \u{00b7} 4.2s"
        );
        assert_eq!(meta_suffix("", "4.2s", 1000), " \u{00b7} 4.2s");
        assert_eq!(meta_suffix("", "", 1000), "");
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
        assert_eq!(fmt_elapsed(900), "0.9s");
        assert_eq!(fmt_elapsed(3_200), "3.2s");
        // Deviation from the design doc's `MmSSs` bucket past 99s: kept as a
        // plain decimal-seconds string for consistency with the header's
        // existing turn-duration format (see `fmt_elapsed`'s doc comment).
        assert_eq!(fmt_elapsed(61_000), "61.0s");
        assert_eq!(fmt_elapsed(125_000), "125.0s");
    }

    #[test]
    fn meta_suffix_drops_the_task_tag_but_keeps_latency() {
        let s = meta_suffix("", "task:3 \u{00b7} 0.0s", 1000);
        assert_eq!(s, " \u{00b7} 0.0s");
        assert!(!s.contains("task:"), "tag must not leak into the tail: {s}");
        // Tag-only meta yields no latency part.
        assert_eq!(meta_suffix("", "task:3", 1000), "");
    }
}
