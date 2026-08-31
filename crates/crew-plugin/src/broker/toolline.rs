//! How a tool call reads in the transcript.
//!
//! Every call is already logged as a hop, so the pane shows what an agent is
//! doing to your machine — but it showed the WIRE FORM:
//!
//! ```text
//! [tool] sys:write_file {"path": "src/lib.rs", "content": "use std::fmt;\nuse …
//! ```
//!
//! The one thing a reader wants (which file? which command?) is buried in
//! punctuation, and for `write_file` the line is mostly the first 150 bytes of
//! the file being written. The same call reads:
//!
//! ```text
//! [tool] sys:write_file  src/lib.rs
//! ```
//!
//! Nothing is hidden that a reader was getting — the arguments were clipped at
//! 200 characters anyway, so a long call was already a truncated blob.

/// Argument names that ARE the call, in the order they should win. `cmd` and
/// `path` cover the built-in `sys` surface; the rest are the conventional
/// spellings an MCP server is likely to use for its subject.
const HEADLINE: &[&str] = &["cmd", "command", "path", "file", "url", "query", "pattern"];

/// The transcript line for one tool call: `server:tool  <subject>`.
///
/// Falls back to the raw arguments when nothing identifiable is there, so a
/// tool this knows nothing about still shows what it was given rather than
/// nothing at all.
pub(crate) fn call_line(label: &str, args: &str, cap: usize) -> String {
    match subject(args) {
        Some(s) => format!("{label}  {}", clip(&s, cap)),
        None => {
            let a = args.trim();
            if a.is_empty() || a == "{}" {
                label.to_string()
            } else {
                format!("{label} {}", clip(a, cap))
            }
        }
    }
}

/// The argument worth showing, if the call has one.
fn subject(args: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(args).ok()?;
    let obj = v.as_object()?;
    for key in HEADLINE {
        if let Some(s) = obj.get(*key).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return Some(s.trim().to_string());
            }
        }
    }
    // A single-field call is unambiguous whatever the field is called.
    if obj.len() == 1 {
        if let Some(s) = obj.values().next().and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return Some(s.trim().to_string());
            }
        }
    }
    None
}

/// Clip to `cap` characters (not bytes — this is display text) with an
/// ellipsis, and flatten newlines: a transcript line is one line.
fn clip(s: &str, cap: usize) -> String {
    let flat: String = s
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    if flat.chars().count() <= cap {
        return flat;
    }
    let head: String = flat.chars().take(cap).collect();
    format!("{head}\u{2026}")
}

#[cfg(test)]
#[path = "toolline_tests.rs"]
mod tests;
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

/// The result line: `sys:run ✓ 1.2s`, the header a folded result card shows.
pub(crate) fn result_line(label: &str, ok: bool, ms: u64) -> String {
    let mark = if ok { '\u{2713}' } else { '\u{2717}' };
    format!("{label} {mark} {}", took(ms))
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

    #[test]
    fn a_result_line_says_outcome_and_duration() {
        assert_eq!(result_line("sys:run", true, 1_200), "sys:run \u{2713} 1s");
        assert_eq!(result_line("fs:read", false, 300), "fs:read \u{2717} 0.3s");
    }
}
