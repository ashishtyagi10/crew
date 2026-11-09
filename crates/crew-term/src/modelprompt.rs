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
use alacritty_terminal::term::TermMode;

use super::TermCore;

impl TermCore {
    /// Feed `bytes`, cut at the `prompts` offsets (just past each prompt
    /// mark), leaving the alternate screen at each cut.
    pub(super) fn feed_split(&mut self, bytes: &[u8], prompts: &[usize]) {
        let mut from = 0;
        for &at in prompts {
            self.feed_part(&bytes[from..at]);
            from = at;
            if self.term.mode().contains(TermMode::ALT_SCREEN) {
                self.feed_part(b"\x1b[?1049l");
            }
        }
        self.feed_part(&bytes[from..]);
    }
}
