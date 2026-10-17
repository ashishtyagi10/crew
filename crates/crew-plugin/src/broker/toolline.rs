//! How long a tool took, as the turn's timeline writes it.
//!
//! The call and result lines themselves are the pane's now: the API engine
//! sends its tools on the same lane as Claude Code's (`toolhive`), and the
//! pane's tool line reads the command or path out of the arguments.

/// `0.4s` / `12s` / `2m04` — a tool's duration, as long as it needs.
///
/// A tool is the one part of a turn whose wait is not the model's, and the
/// only one with a two-minute ceiling (`sysrun::DEFAULT_TIMEOUT_MS`). Without
/// a number on the line, a slow call and a hung one look identical to the
/// person watching, which is the moment they reach for Esc.
pub(crate) fn took(ms: u64) -> String {
    match ms / 1000 {
        0 => format!("{:.1}s", ms as f64 / 1000.0),
        s @ 1..=59 => format!("{s}s"),
        s => format!("{}m{:02}", s / 60, s % 60),
    }
}

#[cfg(test)]
mod took_tests {
    use super::*;

    #[test]
    fn sub_second_keeps_a_decimal_and_never_reads_as_zero() {
        assert_eq!(took(0), "0.0s");
        assert_eq!(took(430), "0.4s");
        assert_eq!(took(999), "1.0s");
    }

    #[test]
    fn seconds_and_minutes_drop_the_decimal() {
        assert_eq!(took(1_000), "1s");
        assert_eq!(took(59_000), "59s");
        assert_eq!(took(64_000), "1m04");
        // The sys:run ceiling, which is the number this exists to make visible.
        assert_eq!(took(120_000), "2m00");
    }
}
