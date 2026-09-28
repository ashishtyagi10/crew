//! Clipping tool results for the agent-facing exchange log: unlike
//! [`super::route::clip`] (whitespace-flattening, for short display
//! strings), this preserves newlines and — when the text must be cut —
//! always keeps the final line intact. Continuation protocols (e.g.
//! `sys:read_file`'s "continue with {\"offset\": N}") put their payload on
//! the last line, so losing it would strand the agent mid-read. Only
//! hard-caps (dropping the final line) when that line alone would already
//! overflow the budget — worst case otherwise is ~`max` + the marker's
//! width. All lengths here are in `char`s (not bytes), so multibyte text
//! never hits a byte-boundary panic.
pub(super) fn clip_result(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    const MARKER: &str = "\n\u{2026} (result clipped) \u{2026}\n";
    let last_line = text.rsplit('\n').next().unwrap_or("");
    let last_line_len = last_line.chars().count();
    if last_line_len > max {
        return text.chars().take(max).chain(['\u{2026}']).collect();
    }
    let reserved = MARKER.chars().count() + last_line_len;
    let head_budget = max.saturating_sub(reserved);
    let head: String = text.chars().take(head_budget).collect();
    format!("{head}{MARKER}{last_line}")
}

/// Chars of one tool result the agent's next prompt carries: the relay's
/// exchange log, and the same number as crew-hive's `RESULT_CAP` for the
/// swarm. `sysread::PAGE` is sized from it, so a `sys:read_file` page and
/// its note always arrive whole; raising the page means raising this.
pub(super) const AGENT_CLIP: usize = 6_000;

/// Bytes a `sys:run` result is fitted to before it leaves `sysrun`
/// (`runfit`), for the reason `sysread::PAGE` is 5,600: whatever is longer
/// than [`AGENT_CLIP`] is cut by clips that keep the START, and a build or a
/// test run says what went wrong at the END. Fitted under the clip, the
/// verdict is chosen here rather than lost there. Bytes, not chars, so a
/// fitted result is under the clip whatever it is written in; 400 left over
/// for nothing to go wrong in.
#[cfg_attr(not(unix), allow(dead_code))]
pub(super) const RUN_FIT: usize = 5_600;

const _: () = assert!(RUN_FIT < AGENT_CLIP);
