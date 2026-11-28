//! The window title stack (`CSI 22 ; Ps t` push, `CSI 23 ; Ps t` pop): vim,
//! tmux, less and htop save the title on entry and restore it on exit, so
//! the pane goes back to the shell's name instead of keeping theirs.
//! Child module of `modelwipe`, whose scanner finds the pushes and pops.
//!
//! alacritty_terminal keeps the stack and sends a popped title as an
//! ordinary `Event::Title` (so the pane follows it with no other wiring).
//! Three things it leaves to crew:
//!
//! - Icon-only (`Ps` = 1). It treats every push and pop alike, but an icon
//!   name is not the title crew shows. Those are aborted (final byte
//!   replaced by CAN), as xterm leaves the title alone for them.
//! - Depth. Its stack goes 4096 deep; xterm's holds 10. crew counts its own
//!   depth, capped at [`CAP`] (a push past it still goes through, the oldest
//!   is forgotten: the newest titles are the ones a program pops), and aborts
//!   a pop it holds nothing for, so alacritty's stale bottom entries are
//!   never reached.
//! - Leftovers. A program killed after its push never pops, and the shell
//!   would keep that program's title. [`TermCore::title_unwind`] pops what
//!   crew holds where the shell has the pane back, which leaves the
//!   BOTTOM-MOST title, the one saved before the program started (clearing
//!   the stack alone would leave the dead program's title showing). The
//!   shell's own `OSC 0`, set after that, replaces it as usual.
//!
//! A pane the user named keeps that name: the name is read before the
//! program's title (`Pane::title_text`) and nothing here touches it.
use super::TermCore;

/// How many titles a stack keeps, as xterm's.
pub(crate) const CAP: u8 = 10;

impl TermCore {
    /// A push is about to complete (alacritty saves the current title).
    pub(crate) fn title_push(&mut self) {
        let d = &mut self.wipes.title_depth;
        *d = (*d + 1).min(CAP);
    }

    /// A pop is about to complete: whether crew holds a title for it.
    pub(crate) fn title_pop(&mut self) -> bool {
        let d = &mut self.wipes.title_depth;
        let held = *d > 0;
        *d = d.saturating_sub(1);
        held
    }

    /// RIS forgets the stack (alacritty's own is emptied by it too).
    pub(crate) fn title_reset(&mut self) {
        self.wipes.title_depth = 0;
    }

    /// Pop every title a dead program pushed, ending on the first one saved.
    /// Returns whether there was any.
    pub(crate) fn title_unwind(&mut self) -> bool {
        let n = usize::from(self.wipes.title_depth);
        if n > 0 {
            self.feed(&b"\x1b[23;0t".repeat(n));
        }
        n > 0
    }
}

#[cfg(test)]
#[path = "modeltitle_tests.rs"]
mod tests;
