//! The Far command bar's `!` AI ask: `! <description>` submits a one-shot
//! provider call (`crew_plugin::suggest_far_command`, Task 1) on a worker
//! thread; `FarPane::poll_ask` (this module's `mod.rs` half) drains the
//! result each tick, the same shape as `run.rs`'s `running`/`poll_cmd`, so
//! the winit thread never blocks on the network. The reply REPLACES the
//! bar's content as an editable, highlighted suggestion — Enter runs it via
//! the normal `run_cmdline` path, Esc restores the original `!` text, and
//! further typing just edits it like ordinary text. Distinct from
//! `crate::app`'s top-level `!command` (`bang_command`, spawns a whole
//! pane) — same prefix character, unrelated feature.
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// Provider deadline for one ask — bounded so a dead network resolves to a
/// status line, not a forever-pending "thinking…".
pub(crate) const ASK_TIMEOUT: Duration = Duration::from_secs(20);

/// The Far pane's in-flight or landed `!` ask.
pub(crate) enum AskState {
    /// Waiting on the worker thread; `FarPane::cmdline` still shows the
    /// typed `! <description>` text untouched.
    Thinking {
        started: Instant,
        rx: Receiver<Result<String, String>>,
    },
    /// A suggestion landed and replaced `cmdline`; `original` is the `!
    /// <description>` text Esc restores.
    Suggested { original: String },
}

/// If `line` is a `! <description>` AI ask, return the trimmed description
/// (empty when just `!` or `!` followed only by whitespace); else `None`.
/// Checked in `keys.rs`'s Enter handling before falling back to
/// `run_cmdline`, the same explicit-prefix pattern as `crate::app`'s
/// `bang_command`/`star_command`.
pub(crate) fn bang_ask(line: &str) -> Option<&str> {
    line.strip_prefix('!').map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bang_ask_parses_the_description() {
        assert_eq!(bang_ask("! list rust files"), Some("list rust files"));
        assert_eq!(bang_ask("!  kill port 8080 "), Some("kill port 8080"));
        assert_eq!(bang_ask("!"), Some(""));
        assert_eq!(bang_ask("!   "), Some(""));
    }

    #[test]
    fn lines_without_a_leading_bang_are_not_an_ask() {
        assert_eq!(bang_ask("ls -la"), None);
        assert_eq!(bang_ask("echo hi!"), None);
        assert_eq!(bang_ask(""), None);
    }
}
