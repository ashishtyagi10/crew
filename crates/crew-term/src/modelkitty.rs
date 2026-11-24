//! The kitty keyboard protocol's mode stacks, kept from crashing crew and
//! from outliving the program that pushed them. Child module of `modelwipe`,
//! whose scanner finds the pushes and pops.
//!
//! alacritty_terminal keeps the stacks (one per screen, as the spec asks),
//! answers `CSI ? u` and sets the mode bits the key encoder reads
//! (`keykitty` in the app). Two things it leaves to crew:
//!
//! - Depth. At 4096 entries its push removes the oldest entry of the TITLE
//!   stack instead, which panics when no title was pushed: `cat` of a file
//!   holding 4096 `ESC [ > 1 u` took the whole app down. crew counts each
//!   screen's depth itself and drops a push past [`CAP`] (its final byte is
//!   replaced by CAN, which aborts the sequence). No program nests that deep.
//! - Leftovers. A program killed in kitty mode (`kill -9` on nvim or Claude
//!   Code) never pops, and the shell then gets `ESC [ 99 ; 5 u` for Ctrl+C.
//!   [`POP_ALL`] empties the stack where the shell has the pane back:
//!   in-stream at the first OSC 133 prompt mark after a command (`C` or `D`
//!   mark; `modelprompt`), and from the foreground poll only for a shell
//!   that sends no command marks (`promptmodes`). Not at every prompt mark: a
//!   shell may ask for the protocol itself once its prompt is drawn (fish 4
//!   sends `CSI = 5 u` once per command line) and redraw the prompt, mark
//!   and all, on a resize; popping there would take the shell's own away.
use alacritty_terminal::term::TermMode;

use super::TermCore;

/// The deepest crew lets either stack grow (alacritty breaks at 4096).
pub(crate) const CAP: u16 = 256;

/// Pop every entry of the active screen's stack: a count past [`CAP`].
pub(crate) const POP_ALL: &[u8] = b"\x1b[<65535u";

/// Per screen (primary, alternate), how deep its stack is, and the count of
/// command marks at the last prompt that popped (`None` before the first).
#[derive(Default)]
pub(crate) struct KittyStacks {
    depth: [u16; 2],
    popped_at: Option<u64>,
}

impl TermCore {
    fn screen(&self) -> usize {
        usize::from(self.term.mode().contains(TermMode::ALT_SCREEN))
    }

    /// A push is about to complete (the parser stopped before its final
    /// byte): whether it may. Counted against the screen it lands on.
    pub(crate) fn kitty_push(&mut self) -> bool {
        let s = self.screen();
        let d = &mut self.wipes.kitty.depth[s];
        let ok = *d < CAP;
        *d += u16::from(ok);
        ok
    }

    /// A pop of `n` entries is about to complete.
    pub(crate) fn kitty_pop(&mut self, n: u32) {
        let s = self.screen();
        let d = &mut self.wipes.kitty.depth[s];
        *d = d.saturating_sub(u16::try_from(n).unwrap_or(u16::MAX));
    }

    /// A full reset (RIS) empties both stacks.
    pub(crate) fn kitty_reset(&mut self) {
        self.wipes.kitty.depth = [0; 2];
    }

    /// At a prompt mark: whether to empty the stacks, true at the first
    /// prompt and then once after each command.
    pub(crate) fn kitty_pop_due(&mut self) -> bool {
        let now = Some(self.osc7.commands());
        let due = self.wipes.kitty.popped_at != now;
        self.wipes.kitty.popped_at = now;
        due
    }

    /// Whether the shell marks its commands (OSC 133 `C` / `D`), so the
    /// prompt cut empties what a dead program pushed.
    pub(crate) fn command_marks(&self) -> bool {
        self.osc7.commands() > 0
    }
}

#[cfg(test)]
#[path = "modelkitty_tests.rs"]
mod tests;
