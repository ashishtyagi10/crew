//! The one paste that runs before you have read it.
//!
//! A terminal sends what you paste as if you had typed it, newlines included
//! — so a pasted block whose first line ends in a newline runs immediately,
//! and whatever follows runs after it. That is the oldest footgun in the
//! terminal, and the reason every serious terminal asks first.
//!
//! Crew asks only when the answer matters. A program that enabled **bracketed
//! paste** (every modern shell, every editor, every agent CLI) receives the
//! block wrapped and decides for itself — nothing runs, so there is nothing to
//! warn about. Without it, a multi-line paste is a sequence of commands.
use std::time::{Duration, Instant};

/// How long a held paste waits for a second Cmd+V before it is forgotten. Long
/// enough to read the question, short enough that an answer you have forgotten
/// giving cannot fire later.
const HOLD: Duration = Duration::from_secs(15);

/// Whether pasting `text` into a pane with this bracketed-paste state should
/// be held for confirmation.
///
/// One trailing newline is the common, harmless case — copying a line out of
/// a file takes its terminator with it — and holding it would train people to
/// confirm everything. What matters is a newline with something after it.
pub(crate) fn needs_confirm(text: &str, bracketed: bool) -> bool {
    !bracketed && text.trim_end_matches(['\n', '\r']).contains('\n')
}

/// How many lines a held paste would run, for the question crew asks.
pub(crate) fn line_count(text: &str) -> usize {
    text.trim_end_matches(['\n', '\r']).lines().count()
}

/// A paste waiting for a second Cmd+V.
#[derive(Default)]
pub(crate) struct Held {
    text: Option<String>,
    at: Option<Instant>,
}

impl Held {
    /// Hold `text`, replacing anything already held — the newer clipboard is
    /// the one you meant.
    pub(crate) fn hold(&mut self, text: &str, now: Instant) {
        self.text = Some(text.to_string());
        self.at = Some(now);
    }

    /// Take the held paste if it is still fresh. Anything older than
    /// [`HOLD`] is dropped rather than sent: a confirmation you have
    /// forgotten giving is not a confirmation.
    pub(crate) fn take(&mut self, now: Instant) -> Option<String> {
        let at = self.at.take()?;
        let text = self.text.take()?;
        (now.saturating_duration_since(at) <= HOLD).then_some(text)
    }

    pub(crate) fn clear(&mut self) {
        self.text = None;
        self.at = None;
    }
}

#[cfg(test)]
#[path = "pastesafe_tests.rs"]
mod tests;
