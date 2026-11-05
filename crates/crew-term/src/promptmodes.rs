//! Input modes a dead program left on, dropped when the shell has the pane
//! back (child module of `pty`).
//!
//! A TUI that dies without restoring the terminal (`kill -9`, a crash, fzf
//! killed mid-pick) leaves its mouse reporting and focus reporting switched
//! on. The shell never turns them off, so every wheel tick at the prompt
//! wrote `ESC [ < 64 ; x ; y M` into the command line, a click wrote another,
//! and switching panes typed `ESC [ I`. Crew already knows when the shell has
//! the pane again (the foreground job is the shell, or the prompt's OSC 133
//! mark), and no shell turns these modes on itself, so they are switched off
//! there: fed to the parser, not written to the program.
//!
//! Only on the primary screen. A pane whose command is a program and not a
//! shell (an agent run straight in the pane) also reads as "the shell has it"
//! when a job it ran ends, and on the alternate screen that program may still
//! be the one that asked for the mouse. Keys stay as they are: oh-my-zsh
//! turns application cursor keys (DECCKM) on itself at every prompt, and
//! bracketed paste is the shell's own.
use crate::modes::InputModes;

use super::PtyTerm;

/// Mouse tracking (clicks, drags, any motion), its UTF-8 and SGR encodings
/// and focus reporting: every one of them the parser tracks, all reset.
const RESET: &[u8] = b"\x1b[?1000;1002;1003;1005;1006;1004l";

/// The bytes that clear what a dead program left in `m`, or `None` when
/// there is nothing to clear or a full-screen program may still own it.
pub fn leftover_reset(m: InputModes) -> Option<&'static [u8]> {
    (!m.alt_screen && (m.mouse || m.focus_events)).then_some(RESET)
}

impl PtyTerm {
    /// The shell has the pane back: drop the mouse and focus reporting a
    /// program left on (see the module doc). Returns whether any was on.
    pub fn release_leftover_modes(&mut self) -> bool {
        let Some(reset) = leftover_reset(self.core.input_modes()) else {
            return false;
        };
        self.core.feed(reset);
        true
    }
}

#[cfg(test)]
#[path = "promptmodes_tests.rs"]
mod tests;
