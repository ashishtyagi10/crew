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
//! Crew's own DECSET 2031 flag (colour-scheme notifications, sniffed by
//! `schemenotify`) is one of them: Claude Code turns it on at start and off
//! only on a clean exit, so after a `kill -9` every light/dark flip of the
//! theme wrote `ESC [ ? 997 ; 1 n` into the shell (zsh: `command not found:
//! 997`). It goes with the mouse, through the same parser feed.
//!
//! So does the cursor. htop, fzf, less, vim and Claude Code all hide it
//! (`ESC [ ? 25 l`) and show it again only on a clean exit; after a
//! `kill -9` the prompt came back with no cursor at all, and no shell shows
//! it again. A hidden cursor with the shell back is a leftover like the
//! mouse: shown again by the same reset.
//!
//! Only on the primary screen. A pane whose command is a program and not a
//! shell (an agent run straight in the pane) also reads as "the shell has it"
//! when a job it ran ends, and on the alternate screen that program may still
//! be the one that asked for the mouse. A shell's OSC 133 prompt mark on the
//! alternate screen first takes the pane back to the primary one
//! (`model::modelprompt`), so the reset that mark runs reaches a crashed
//! full-screen program's mouse and cursor too (leaving the alternate screen
//! alone never shows the cursor again). Keys stay as they are: oh-my-zsh
//! turns application cursor keys (DECCKM) on itself at every prompt, and
//! bracketed paste is the shell's own.
use crate::modes::InputModes;

use super::PtyTerm;

/// Mouse tracking (clicks, drags, any motion), its UTF-8 and SGR encodings
/// and focus reporting: every one of them the parser tracks, all reset; and
/// 2031, which only crew's scheme scanner tracks (it sees the same feed);
/// then the cursor shown (DECTCEM).
const RESET: &[u8] = b"\x1b[?1000;1002;1003;1005;1006;1004;2031l\x1b[?25h";

/// The bytes that clear what a dead program left in `m`, or `None` when
/// there is nothing to clear or a full-screen program may still own it.
/// `scheme` is whether DECSET 2031 is on and `hidden` whether the cursor is
/// hidden (neither is an `InputModes` field: nothing routes input by them).
pub fn leftover_reset(m: InputModes, scheme: bool, hidden: bool) -> Option<&'static [u8]> {
    (!m.alt_screen && (m.mouse || m.focus_events || scheme || hidden)).then_some(RESET)
}

impl PtyTerm {
    /// The shell has the pane back: drop the mouse, focus and scheme reporting a
    /// program left on and show the cursor it hid (see the module doc).
    /// Returns whether any was left.
    pub fn release_leftover_modes(&mut self) -> bool {
        let scheme = self.core.scheme_notify_enabled();
        let hidden = self.core.cursor_hidden();
        let Some(reset) = leftover_reset(self.core.input_modes(), scheme, hidden) else {
            return false;
        };
        self.core.feed(reset);
        true
    }
}

#[cfg(test)]
#[path = "promptmodes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "promptcursor_tests.rs"]
mod cursor_tests;
