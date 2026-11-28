//! In-band resize reports (DECSET 2048), sent where the pty is resized.
//!
//! The spec wants the report to follow the winsize change, so a program that
//! reads `TIOCGWINSZ` on receipt sees the size it was told. Both ways the
//! pty's size moves (the grid in [`PtyTerm::resize_pty`], the cell size in
//! `set_cell_px`) end in [`PtyTerm::sync_pty`], which resizes the pty first
//! and only then queues the report. The app resizes once per relayout, not
//! per drag frame, so one report goes out per settled size; a call that
//! leaves the size as it was reports nothing.
use portable_pty::PtySize;

use super::super::PtyTerm;

impl PtyTerm {
    /// Resize the grid and the pty, reporting the new size if it changed.
    pub(super) fn resize_pty(&mut self, size: crate::GridSize) {
        let before = self.core.pty_size();
        self.core.resize(size);
        let _ = self.master.resize(self.core.pty_size());
        self.report_inband(before);
    }

    /// The cell size moved: tell the pty if the text area's pixels changed.
    pub(super) fn sync_pty(&mut self, before: PtySize) {
        if self.core.pty_size() != before {
            let _ = self.master.resize(self.core.pty_size());
            self.report_inband(before);
        }
    }

    fn report_inband(&self, before: PtySize) {
        if self.core.inband_enabled() && self.core.pty_size() != before {
            self.core.push_inband_report();
        }
    }
}

#[cfg(test)]
#[path = "ptyinband_tests.rs"]
mod tests;
