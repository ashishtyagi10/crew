//! Commands that ask once before doing something you cannot undo.
//!
//! `/closeall` ends every session in the window; `/only` ends all but one.
//! Both are one keystroke away from `/close` and `/out` in a fuzzy palette,
//! and neither can be taken back — a closed pane takes its scrollback, its
//! running command and its agent with it.
//!
//! The ask is the same shape as the held paste: run it again and it happens.
//! Not a dialog — crew has no modal to put one in, and a command that answers
//! its own confirmation with the same keystroke that asked for it is one you
//! can learn without reading anything.
use std::time::{Duration, Instant};

/// How long a pending confirmation stands. Long enough to read the question
/// and answer it, short enough that the answer cannot arrive from a command
/// you have forgotten typing.
const WINDOW: Duration = Duration::from_secs(10);

/// The command awaiting a second run.
#[derive(Default)]
pub(crate) struct Pending {
    cmd: Option<String>,
    at: Option<Instant>,
}

impl Pending {
    /// Whether `cmd` has already been asked for and may run now. A different
    /// command replaces the pending one rather than answering it — that is
    /// the case this exists to catch.
    pub(crate) fn answered(&mut self, cmd: &str, now: Instant) -> bool {
        let fresh = self
            .at
            .is_some_and(|at| now.saturating_duration_since(at) <= WINDOW);
        let same = self.cmd.as_deref() == Some(cmd);
        if fresh && same {
            self.clear();
            return true;
        }
        self.cmd = Some(cmd.to_string());
        self.at = Some(now);
        false
    }

    pub(crate) fn clear(&mut self) {
        self.cmd = None;
        self.at = None;
    }
}

#[cfg(test)]
#[path = "confirm_tests.rs"]
mod tests;
