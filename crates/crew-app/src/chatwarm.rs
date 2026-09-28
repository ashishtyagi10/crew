//! The composer telling the broker a message is on its way.
//!
//! The first key of a message sends `PluginCommand::Warm`, and the broker
//! opens the provider connection while the rest is typed — so the turn's
//! first model call skips the ~0.5 s of TCP and TLS it was measured paying
//! whenever the pooled socket had idled out (the broker's `prewarm` says the
//! rest). Codex and Claude Code do the same: connect while the user composes.
//!
//! Only on the EMPTY → non-empty edge, and at most once per 20 s per pane. A
//! hint per key would be a line on the broker's stdin per key, for a socket
//! the pool keeps 30 s anyway.
use std::time::{Duration, Instant};

use crate::chat::ChatPane;
use crate::chatkeys::{ChatAction, ChatInput};
use crew_plugin::PluginCommand;

/// At most one warm per pane this often — under the pool's 30 s idle limit,
/// so a message typed slowly still finds the socket open when it is sent.
pub(crate) const EVERY: Duration = Duration::from_secs(20);

/// When this pane last asked for a warm.
#[derive(Debug, Default)]
pub(crate) struct WarmLatch {
    last: Option<Instant>,
}

impl WarmLatch {
    /// Whether the composer going from `was_empty` to `is_empty` at `now` is
    /// the start of a message worth a warm, recording it when it is.
    pub(crate) fn edge(&mut self, was_empty: bool, is_empty: bool, now: Instant) -> bool {
        if !was_empty || is_empty {
            return false;
        }
        if self
            .last
            .is_some_and(|t| now.saturating_duration_since(t) < EVERY)
        {
            return false;
        }
        self.last = Some(now);
        true
    }
}

impl ChatPane {
    /// [`Self::on_input`] for a key the user pressed, plus the warm when that
    /// key started a message. Around `on_input` rather than inside it: the
    /// composer fills from several places in there (a typed character, a
    /// recalled line, a taken ghost, a filled palette row), and every one of
    /// them is the same edge seen from outside.
    pub(crate) fn on_typed(&mut self, k: ChatInput, cwd: &std::path::Path) -> Option<ChatAction> {
        let was_empty = self.input.is_empty();
        let out = self.on_input(k, cwd);
        self.warm_on_edge(was_empty, Instant::now());
        out
    }

    /// Pasted text into the composer — the other way a message starts.
    pub(crate) fn paste_composer(&mut self, text: &str) {
        let was_empty = self.input.is_empty();
        self.input.push_str(text);
        self.warm_on_edge(was_empty, Instant::now());
    }

    /// Send the warm if the composer just filled and the latch allows it.
    /// A failed write is dropped: the hint is worth nothing once the broker
    /// is gone, and the send that follows reports the dead pipe.
    fn warm_on_edge(&mut self, was_empty: bool, now: Instant) {
        if self.warm.edge(was_empty, self.input.is_empty(), now) {
            let _ = self.plugin.send(&PluginCommand::Warm {});
        }
    }
}

#[cfg(test)]
#[path = "chatwarm_tests.rs"]
mod tests;
