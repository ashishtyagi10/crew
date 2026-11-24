//! Which job has the pane: the shell, or the program it ran (child module
//! of `pty`).
//!
//! The answer names the pane's running program in its title, opens and
//! closes the `/out` spans, raises "finished" notifications and, on the
//! edge back to the shell, runs the leftover-mode reset (`promptmodes`).
//! It came from `tcgetpgrp`, and a failed call read as "the shell is at its
//! prompt": under a plain zsh one transient failure while Claude Code ran
//! was a command ending, and the reset popped its kitty keyboard flags and
//! showed the cursor it hid. A lookup that fails now keeps the last answer.

use super::PtyTerm;

/// The foreground job from `group` (the tty's foreground process group, or
/// `None` when the lookup failed), the shell's own pid and the `last`
/// answer: `None` when the shell leads its own group (at its prompt),
/// `last` when the OS couldn't say.
pub(crate) fn settle(group: Option<u32>, shell: Option<u32>, last: Option<u32>) -> Option<u32> {
    match group {
        None => last,
        Some(fg) if Some(fg) == shell => None,
        Some(fg) => Some(fg),
    }
}

impl PtyTerm {
    /// PID of the foreground command running in this pane — the process group in
    /// control of the tty. `None` when the shell itself is at its prompt (so the
    /// pane is idle) or on a platform that doesn't expose it. Lets the title name
    /// the running program (e.g. `claude`, `codex`).
    pub fn foreground_pid(&self) -> Option<u32> {
        // `process_group_leader` is a Unix-only portable-pty API; Windows has no
        // tty foreground-process-group concept, so the pane is simply never
        // labelled with a running command there (no group, no last answer).
        #[cfg(unix)]
        let group = self
            .master
            .process_group_leader()
            .and_then(|g| u32::try_from(g).ok());
        #[cfg(not(unix))]
        let group = None;
        let fg = settle(group, self.child.process_id(), self.fg_last.get());
        self.fg_last.set(fg);
        fg
    }
}

#[cfg(test)]
#[path = "fgpid_tests.rs"]
mod tests;
