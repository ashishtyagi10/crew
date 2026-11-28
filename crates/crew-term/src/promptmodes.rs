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
//! alone never shows the cursor again). Keys mostly stay as they are:
//! oh-my-zsh turns application cursor keys (DECCKM) on itself at every
//! prompt, and bracketed paste is the shell's own. The one key mode reset is
//! the kitty keyboard protocol a killed nvim or Claude Code pushed (Ctrl+C
//! at the prompt typed `ESC [ 99 ; 5 u`), and only on the foreground poll's
//! shell back for a shell that sends no OSC 133 command marks: for one that
//! does, the stack was already emptied at its first prompt mark after the
//! command, and a shell that turns the protocol on for itself after that
//! (fish 4) keeps it (`model::modelkitty`). Never on the prompt mark's reset
//! ([`PtyTerm::release_prompt_leftovers`]): before its first command a fish 4
//! or nushell counts no command marks, and that reset popped the `CSI = 5 u`
//! the shell had just sent; a command that did run since the last prompt
//! was already popped in-stream at that same mark.
use crate::modes::InputModes;

use super::PtyTerm;

/// Mouse tracking (clicks, drags, any motion), its UTF-8 and SGR encodings
/// and focus reporting: every one of them the parser tracks, all reset; and
/// 2031 and 2048 (in-band resize: a killed neovim left `ESC [ 48 ; … t` for
/// the shell on every resize), which only crew's scheme scanner tracks (same
/// feed); and 1016 (SGR-Pixels, `pixelmouse`, likewise scanner-only);
/// then the cursor shown (DECTCEM).
const RESET: &[u8] = b"\x1b[?1000;1002;1003;1005;1006;1004;1016;2031;2048l\x1b[?25h";

/// OSC 22 with an empty name: the default pointer.
const CLEAR_SHAPE: &[u8] = b"\x1b]22;\x1b\\";

/// [`RESET`], then every kitty keyboard push popped (`modelkitty::POP_ALL`).
const RESET_KITTY: &[u8] =
    b"\x1b[?1000;1002;1003;1005;1006;1004;1016;2031;2048l\x1b[?25h\x1b[<65535u";

/// The bytes that clear what a dead program left in `m`, or `None` when
/// there is nothing to clear or a full-screen program may still own it.
/// `scheme` is whether DECSET 2031 is on and `hidden` whether the cursor is
/// hidden (neither is an `InputModes` field: nothing routes input by them);
/// `marked` whether the shell sends OSC 133 command marks (then the prompt
/// mark empties the kitty keyboard stack, and this reset leaves it be), or
/// this is the prompt mark's own reset (it leaves the stack be too).
pub fn leftover_reset(
    m: InputModes,
    scheme: bool,
    hidden: bool,
    marked: bool,
) -> Option<&'static [u8]> {
    if m.alt_screen {
        return None;
    }
    if m.kitty_keys && !marked {
        return Some(RESET_KITTY);
    }
    (m.mouse || m.pixel_mouse || m.focus_events || scheme || hidden).then_some(RESET)
}

impl PtyTerm {
    /// The foreground job is the shell again: drop the mouse, focus and
    /// scheme reporting and the kitty keyboard flags a program left on and
    /// show the cursor it hid (see the module doc).
    /// Returns whether any was left.
    pub fn release_leftover_modes(&mut self) -> bool {
        let marked = self.core.command_marks();
        self.release(marked)
    }

    /// [`Self::release_leftover_modes`] at an OSC 133 prompt mark: all but
    /// the kitty keyboard stack, which is the shell's own by now (the mark's
    /// in-stream cut already popped a dead program's).
    pub fn release_prompt_leftovers(&mut self) -> bool {
        self.release(true)
    }

    /// The pointer shape the program in the pane asked for with OSC 22.
    pub fn pointer_shape(&self) -> Option<&str> {
        self.core.pointer_shape()
    }

    fn release(&mut self, marked: bool) -> bool {
        // 2048 rides the scheme flag: both are scanner-only modes with one reset.
        let scheme = self.core.scheme_notify_enabled() || self.core.inband_enabled();
        let hidden = self.core.cursor_hidden();
        let modes = self.core.input_modes();
        // A killed lazygit must not leave its hand over the prompt: the shape
        // goes with the mouse (not on the alternate screen, where a program
        // may still own it).
        let shape = !modes.alt_screen && self.core.pointer_shape().is_some();
        if shape {
            self.core.feed(CLEAR_SHAPE);
        }
        let Some(reset) = leftover_reset(modes, scheme, hidden, marked) else {
            return shape;
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

#[cfg(test)]
#[path = "promptkitty_tests.rs"]
mod kitty_tests;

#[cfg(all(test, unix))]
#[path = "promptshape_tests.rs"]
mod shape_tests;

#[cfg(test)]
#[path = "promptpixel_tests.rs"]
mod pixel_tests;
