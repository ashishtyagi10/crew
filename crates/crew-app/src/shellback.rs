//! The shell has a terminal pane back: the one place both signals of that
//! land (the foreground job returning to the shell, polled once a second,
//! and the prompt's OSC 133 `A` mark, read as it arrives).
//!
//! Both closed the running command's span and nothing else, so a program
//! that died with mouse reporting on kept it on at the prompt: every wheel
//! tick and click typed a mouse report into the command line, and every
//! pane switch an `ESC [ I`. Now they also drop the input modes the program
//! left behind (`PtyTerm::release_leftover_modes`).
use crate::pane::TermPane;

impl TermPane {
    /// Close the command's span at buffer line `at` (time `ms`), and drop
    /// the mouse and focus reporting it left switched on.
    pub(crate) fn shell_back(&mut self, at: usize, ms: u64) {
        self.spans.close_at(at, ms);
        self.pty.release_leftover_modes();
    }
}

#[cfg(test)]
#[path = "shellback_tests.rs"]
mod tests;
