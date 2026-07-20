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
}
