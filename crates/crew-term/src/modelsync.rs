//! A synchronized update a program never ended is ended for it.
//!
//! A program that opens a synchronized update (DECSET 2026) has its output
//! held back until it closes the update, so a frame is drawn whole. The
//! parser gives the update 150 ms, but the deadline is only a time it
//! records: nothing ends the update when it passes unless the terminal does,
//! and alacritty's own event loop does, crew's did not. A program killed
//! between opening and closing an update (Ctrl+C in a TUI mid-frame) froze
//! the pane, the shell prompt after it included, until 2 MiB more output
//! arrived. The pane's every read tick now ends an update past its deadline.
use std::time::Instant;

use super::TermCore;

impl TermCore {
    /// End a synchronized update whose deadline has passed, drawing what it
    /// held. `true` when one was ended, so the pane redraws.
    pub(crate) fn flush_stale_sync(&mut self) -> bool {
        let due = self
            .parser
            .sync_timeout()
            .sync_timeout()
            .is_some_and(|at| Instant::now() >= at);
        if due {
            self.parser.stop_sync(&mut self.term);
        }
        due
    }
}

#[cfg(test)]
#[path = "modelsync_tests.rs"]
mod tests;
