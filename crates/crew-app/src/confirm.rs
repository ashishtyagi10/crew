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

/// The command awaiting a second run, and the question it asked.
#[derive(Default)]
pub(crate) struct Pending {
    cmd: Option<String>,
    /// The question, kept for as long as the window stands. It used to live
    /// only in a three-second status flash — which guarded a **ten**-second
    /// window, so for seven of those seconds nothing on screen said that
    /// running the command again would close every pane, and nothing said
    /// when the window had shut either. A question you cannot see is not one
    /// you can answer.
    prompt: Option<String>,
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
        self.prompt = None;
        self.at = None;
    }

    /// Remember the question this ask put on screen, so the bar can keep
    /// showing it for the whole window rather than three seconds of it.
    pub(crate) fn asking(&mut self, prompt: impl Into<String>) {
        self.prompt = Some(prompt.into());
    }

    /// The standing question, while one is standing. `None` once it has been
    /// answered or the window has shut — the bar stops saying it in the same
    /// instant the second run stops meaning "yes".
    pub(crate) fn question(&self, now: Instant) -> Option<&str> {
        self.at
            .filter(|at| now.saturating_duration_since(*at) <= WINDOW)
            .and(self.prompt.as_deref())
    }

    /// Whether anything is armed at all — asked by the dispatcher so it can
    /// disarm on the way into an unrelated command.
    pub(crate) fn armed(&self) -> bool {
        self.cmd.is_some()
    }
}

#[cfg(test)]
#[path = "confirm_tests.rs"]
mod tests;
