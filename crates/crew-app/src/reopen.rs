//! Undo close: `/reopen` (Cmd+Shift+T) brings back the pane you just closed.
//!
//! Closing a pane is the one destructive thing on the grid that has no
//! confirmation — `Cmd+W` and the `[x]` button are a single keystroke and a
//! single click, and until now the pane they took was gone with its directory,
//! its file and its place in the grid. `/closeall` and `/only` learned to ask
//! first (v0.18.99), but asking is the wrong answer for the *one-pane* case:
//! the whole point of `Cmd+W` is that it is cheap. The right answer is the
//! browser's — let it be cheap, and let it be undone.
//!
//! ## What comes back, and what does not
//!
//! A reopened pane is a **new pane in the same place**, not a resurrected one.
//! A shell's PTY died when the pane closed: its scrollback, its environment
//! and whatever it was running went with it, and no amount of bookkeeping
//! brings a killed process back. What crew can honestly restore is the thing
//! you would have typed again — the shell in *that* directory, the viewer on
//! *that* file, the `/crew` chat, the todo list — which is exactly the set
//! [`crate::sessionsave::SavedPane`] already describes for `/restore`.
//!
//! Reusing that type is the point. Session restore and undo-close are the
//! same question asked over two different timescales, so a pane kind that
//! learns to survive a quit gets undo-close for free, and one that cannot be
//! described simply is never remembered — [`saved_for`] returns `None` and
//! `/reopen` reaches past it to something it can actually deliver.
//!
//! ## Why a stack and not a slot
//!
//! `/only` and `/closeall` close many panes at once, and a single slot would
//! turn "close the other five" into an undo of one. The stack is bounded at
//! [`DEPTH`] because it holds paths, not panes — nothing is running, nothing
//! is open, and the oldest entry of a long session is a directory you have
//! long since left.
use std::collections::VecDeque;

use crate::pane::{Pane, PaneContent};
use crate::sessionsave::SavedPane;

/// How many closes the stack remembers. Deeper than the grid's six full
/// tiles, so undoing a `/closeall` on a full grid never runs out halfway.
pub(crate) const DEPTH: usize = 8;

/// One closed pane: what to reopen, and what to call it while it is gone.
/// The title is captured at close time because it is the *pane's* name —
/// `dir · cmd`, a `/name`, a viewer's file — and nothing that survives the
/// close could reconstruct it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Closed {
    pub saved: SavedPane,
    pub title: String,
}

/// The undo-close stack: most recently closed at the back, oldest dropped
/// off the front once it is [`DEPTH`] deep.
#[derive(Default)]
pub(crate) struct ClosedStack {
    entries: VecDeque<Closed>,
}

impl ClosedStack {
    /// Remember a pane crew could reopen. Panes with no honest restore —
    /// settings, a swarm view, an agent pane that was never `/crew` — are
    /// simply not remembered, so `/reopen` skips over them to the last pane
    /// it can really bring back rather than failing on the one it cannot.
    pub(crate) fn remember(&mut self, p: &Pane) {
        let Some(saved) = saved_for(p) else { return };
        self.entries.push_back(Closed {
            saved,
            title: p.title_text(),
        });
        while self.entries.len() > DEPTH {
            self.entries.pop_front();
        }
    }

    /// Take the most recently closed pane, if there is one.
    pub(crate) fn take(&mut self) -> Option<Closed> {
        self.entries.pop_back()
    }

    /// How many closes are still undoable.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

/// What a closing pane leaves behind, or `None` when nothing honest can be
/// written down. Mirrors [`crate::app::CrewApp::session_panes`] kind for
/// kind, with one deliberate difference: a shell's directory is the pane's
/// own tracked `dir` (kept current by the OSC 7 the shell emits on every
/// `cd`) rather than a `sysinfo` read of the shell process. The process is
/// about to be reaped — asking the OS where it is standing is a race against
/// its own death, and the pane already knows.
///
/// A second, quieter difference: `session_panes` copies `p.hidden` into the
/// entry, so a pane parked in the nav comes back parked. Undo-close does not.
/// The minimized flag describes where the pane was *stored*, and reopening is
/// an act of wanting to see it — every entry this returns is `min: false`,
/// which `a_minimized_pane_reopens_visible` holds to.
pub(crate) fn saved_for(p: &Pane) -> Option<SavedPane> {
    let sp = match &p.content {
        PaneContent::Terminal(_) => {
            let d = p.dir.as_ref()?;
            SavedPane::shell(d.to_string_lossy().into_owned())
        }
        PaneContent::Far(f) => {
            let loc = f.active_loc();
            if loc.is_remote() {
                SavedPane::far_remote(loc.rclone_addr())
            } else {
                SavedPane::far(loc.local_path()?.to_string_lossy().into_owned())
            }
        }
        PaneContent::Chat(_) if p.label.as_deref() == Some("crew") => SavedPane::crew(),
        // `/about` and `??` open a synthetic temp file that will not exist
        // in any meaningful sense once the pane is gone — the same rule
        // `session_panes` applies, for the same reason.
        PaneContent::View(v) if !v.ephemeral => {
            SavedPane::view(v.path.to_string_lossy().into_owned())
        }
        PaneContent::Todo(_) => SavedPane::todo(),
        _ => return None,
    };
    Some(sp)
}

impl crate::app::CrewApp {
    /// `/reopen` (Cmd+Shift+T) — bring back the most recently closed pane,
    /// through the very spawn path `/restore` uses. The pane lands where a
    /// new pane lands: focused, on the grid, un-minimized.
    ///
    /// The tracked cwd is saved and put back around the call for the same
    /// reason `restore_from` does it — `open_saved` steers `self.cwd` to the
    /// pane's directory, and that steering must not outlive the reopen and
    /// silently become the directory the *next* `Cmd+T` opens in.
    pub(crate) fn reopen_pane(&mut self) {
        let Some(entry) = self.closed.take() else {
            self.set_status("nothing to reopen");
            return;
        };
        let kept = std::mem::take(&mut self.cwd);
        let opened = self.open_saved(&entry.saved, &kept);
        self.cwd = kept;
        // A reopen is a request to look at the pane, so it never lands
        // zoomed on top of a grid the user did not ask to leave — `open_view`
        // sets that flag for a fresh `/view` and this is not one.
        self.zoomed = false;
        if opened {
            let left = self.closed.len();
            let more = match left {
                0 => String::new(),
                n => format!(" ({n} more)"),
            };
            self.set_status(format!("reopened {}{more}", entry.title));
        }
        self.redraw();
    }
}

#[cfg(test)]
#[path = "reopen_tests.rs"]
mod tests;
