//! `/restart`: relaunch crew as a fresh process and come back where you were.
//!
//! The one step of an update crew will not take by itself. Releases download
//! and install in the background ([`crate::autoupdate`]) and then wait; the
//! restart that swaps the running process for the binary on disk is typed —
//! `/restart`, or a press on the nav's RESTART card — because it ends every
//! program running in every pane. The fresh process also re-reads
//! `config.toml`, so external config edits take effect too.
//!
//! A canvas cannot do this alone: the snapshot is of every window, and only
//! [`crate::canvas`] sees them all. So the canvas ASKS (`want_restart`), and
//! this file is its two halves of the answer — what it hands over on the way
//! out, and what it does with that on the way back in.
//!
//! What comes back is what [`crate::reopen`] calls an honest restore: each
//! shell in its directory, each Far panel, viewer, todo and smith pane, in
//! its tab and window, minimized if it was, with the focus where it sat. The
//! processes themselves do not survive — nothing can carry a running program
//! across the death of the terminal it ran in.
use crate::app::CrewApp;
use crate::restartsnap::Win;

impl CrewApp {
    /// This window's panes for the restart snapshot, and which had the focus.
    pub(crate) fn restart_window(&self) -> Win {
        let saved = self.session_panes_indexed();
        let focus = match self.input.focused {
            true => None,
            false => saved.iter().position(|(i, _)| *i == self.focused),
        };
        Win {
            focus,
            panes: saved.into_iter().map(|(_, sp)| sp).collect(),
        }
    }

    /// Reopen a window's panes after a restart and put the focus back.
    /// Returns how many opened.
    pub(crate) fn resume_window(&mut self, win: Win) -> usize {
        let n = win.panes.len();
        if n == 0 {
            return 0;
        }
        let before = self.panes.len();
        self.restore_from(win.panes);
        let opened = self.panes.len() - before;
        // The saved index only means something when every pane opened; a
        // failed spawn shifts the ones after it, and a guess would focus a
        // neighbour. `restore_from` already left a sane focus for that case.
        match win.focus {
            None => self.input.focused = true,
            Some(k) if opened == n && !self.panes[before + k].hidden => {
                self.focused = before + k;
                self.input.focused = false;
            }
            Some(_) => {}
        }
        opened
    }
}

#[cfg(test)]
#[path = "restart_tests.rs"]
mod tests;
