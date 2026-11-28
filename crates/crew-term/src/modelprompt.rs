//! A shell prompt drawn on a dead program's alternate screen takes the
//! pane back to the primary one.
//!
//! A full-screen program killed with `kill -9` (or crashed) never sends the
//! `ESC [ ? 1049 l` that leaves the alternate screen, and the shell never
//! sends it either. The prompt was then drawn on the alternate screen: no
//! scrollback, the program's last frame round it, and since
//! `promptmodes::leftover_reset` (rightly) keeps its hands off the alternate
//! screen, the dead program's mouse modes stayed too — every wheel tick typed
//! a mouse report (or, through 1007, walked shell history).
//!
//! The OSC 133 `A` mark is the signal: only a shell's prompt integration
//! sends it, and a full-screen program that hosts shells (tmux, nvim's
//! `:terminal`, vim's `:terminal`) parses its children's marks and draws
//! their output itself rather than forwarding them. So when a prompt mark
//! ends while the alternate screen is up, crew leaves it there, at that byte,
//! before the prompt's own text lands (switching after the whole chunk would
//! draw the prompt on the screen being thrown away). The app drains the same
//! mark right after and runs the leftover reset, now on the primary screen.
//!
//! Shells without OSC 133 are not helped: the once-a-second foreground poll
//! can't tell a shell at its prompt from a program run as the pane's command.
//!
//! The kitty keyboard stack a dead program pushed is emptied at the same cut,
//! the alternate screen's before it is left (else the next full-screen
//! program would inherit nvim's flags) and the primary's after: at the first
//! prompt after a command, in-stream, so a shell that asks for the protocol
//! itself once its prompt is drawn keeps it (see `modelkitty`).
use alacritty_terminal::term::TermMode;

use super::modelwipe::POP_ALL;
use super::TermCore;
use crate::osc::Cut;

impl TermCore {
    /// Feed `bytes`, cut at the offset just past each OSC 133 mark, and note
    /// where the cursor is at each. At a prompt cut, leave the alternate
    /// screen and empty both kitty keyboard stacks when a command ran since
    /// the last; at a done cut, unwind pushed titles.
    pub(super) fn feed_split(&mut self, bytes: &[u8], cuts: &[(usize, Cut)]) {
        let mut from = 0;
        for &(at, cut) in cuts {
            self.feed_part(&bytes[from..at]);
            from = at;
            match cut {
                // The command is over and the shell's precmd (its fresh
                // title) comes after this mark: pop a dead program's pushes
                // now, so they can't land over it (`modeltitle`).
                Cut::Done => {
                    self.title_unwind();
                }
                Cut::Prompt => self.prompt_cut(),
                Cut::Begin | Cut::Output => {}
            }
            // After the unwind, so a prompt's alternate screen is already
            // gone: the position is the primary's (`markpos`).
            self.mark_here(cut);
        }
        self.feed_part(&bytes[from..]);
    }

    /// A prompt is about to be drawn: leave a dead program's alternate screen
    /// and empty the kitty keyboard stacks when a command ran since the last.
    fn prompt_cut(&mut self) {
        let pop = self.kitty_pop_due();
        if self.term.mode().contains(TermMode::ALT_SCREEN) {
            if pop {
                self.feed_part(POP_ALL);
            }
            self.feed_part(b"\x1b[?1049l");
        }
        if pop {
            self.feed_part(POP_ALL);
        }
    }
}
