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

/// The card header's metadata tail: ` · <rel time>` then ` · <meta>`, each part
/// present only when known. Empty when there's nothing to say.
pub(crate) fn meta_suffix(ts: &str, meta: &str, now_ms: u64) -> String {
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
}
