//! Far Manager pane: a dual-pane file browser (two side-by-side directory
//! listings) spawned by `/far`. Tab switches the active panel when the command
//! line is empty; arrows move the cursor; Enter descends into a folder (or
//! `..`) or opens a file with the OS default. The function-key bar works as
//! labelled: F1 help, F3 views the file in the file-viewer pane, F4 opens it
//! in `$EDITOR` in a terminal pane — both stay inside crew, though a remote
//! file downloads first and falls back to the OS default — F5 copy and
//! F6 move into the other panel, F7 make-folder (a text prompt), F8 delete to
//! trash, F10/Esc close. A Far-style command line sits at the bottom: type a
//! command and press Enter to run it in the active panel's directory — `cd`
//! navigates that panel in place, anything else runs on a worker thread and
//! reloads the listings when it finishes (no new pane is spawned). While
//! typing, Tab completes/cycles the caret token, Up/Down recall persisted
//! history (`far-history`), and fish-style ghost text previews a matching
//! history entry that Right/End accept; Esc cancels an active Tab-cycle
//! first, then clears the typed text, then closes the pane. Lives in the
//! auto-tiling grid like any other pane and renders into a `ratatui` buffer →
//! GPU cells.
pub(crate) use types::*;
mod absorb;
mod ask;
mod cmdhist;
mod cmdline;
mod complete;
mod fileops;
mod icons;
mod keys;
mod list;
mod location;
mod panelchrome;
mod pollcmd;
mod rclone;
mod remote;
mod render;
mod run;
mod sides;
mod types;

use std::path::PathBuf;

use crew_render::CellView;
use winit::event::KeyEvent;

pub use keys::FarAction;
use location::Location;

impl Prompt {
    pub(crate) fn mkdir() -> Self {
        Self {
            kind: PromptKind::MkDir,
            input: String::new(),
        }
    }
}

impl Panel {
    fn new(cwd: PathBuf) -> Self {
        let loc = Location::local(&cwd);
        let entries = list::read_dir(&cwd);
        Self {
            loc,
            entries,
            sel: 0,
            loading: false,
        }
    }

    /// Re-read the current location and clamp the cursor into range. Local
    /// reads synchronously; remote reload is driven asynchronously via
    /// `remote.rs` (a later task) and is a no-op stub here.
    fn reload(&mut self) {
        if let Some(path) = self.loc.local_path() {
            self.entries = list::read_dir(&path);
        }
        self.sel = self.sel.min(self.entries.len().saturating_sub(1));
    }
}

pub struct FarPane {
    pub(crate) left: Panel,
    pub(crate) right: Panel,
    pub(crate) active: Side,
    /// Active text prompt (F7 make-folder), captured before any nav key.
    pub(crate) prompt: Option<Prompt>,
    /// The classic Far command line at the bottom: typed text runs (Enter) as a
    /// command in the active panel's directory. Empty when nothing is typed.
    pub(crate) cmdline: String,
    /// A command started from the command line that is still running on its
    /// worker thread: `(command text, result channel)`.
    pub(crate) running: Option<(String, std::sync::mpsc::Receiver<run::CmdDone>)>,
    /// Persisted command-line history (`far-history`) + Up/Down browse state
    /// and fish-style ghost-text lookups.
    pub(crate) history: cmdhist::CmdHistory,
    /// An in-progress Tab-completion cycle, if any — invalidated by any
    /// edit to `cmdline` (typing, Backspace, running a command).
    pub(crate) complete: Option<complete::CycleState>,
    /// Cached `$PATH` binaries for Command-kind Tab completion. Shared across
    /// every `FarPane` in the process via [`shared_bins`] — the `$PATH`
    /// doesn't change pane to pane, so the scan runs at most once per
    /// session, not once per pane.
    pub(crate) bins: std::sync::Arc<std::sync::OnceLock<Vec<String>>>,
    /// Whether *this pane* has already spawned the `$PATH` scan thread —
    /// guards against spawning one per keystroke before the first scan
    /// lands. Per-pane rather than shared: harmless if another pane's first
    /// Tab also spawns one before the shared cache is filled, since only the
    /// first `OnceLock::set` to land wins and the rest are silently dropped.
    pub(crate) bins_scan_started: bool,
    /// The in-flight or landed `!` AI ask, if any — invalidated (`None`) by
    /// any edit to `cmdline`, same lifecycle rule as `complete`.
    pub(crate) ask: Option<ask::AskState>,
    /// The single in-flight remote (`rclone`) op, if any — a second request
    /// while one is running is rejected with a "busy" status (see
    /// `remote::begin_list`). Landed each tick via `poll_ops`.
    pub(crate) pending: Option<remote::PendingOp>,
    /// The Alt+F1/F2 drive-select overlay, if open — swallows keys until
    /// `choose_drive` (Enter) or a close (Esc) clears it back to `None`.
    pub(crate) drive_select: Option<remote::DriveSelect>,
    /// Downloaded remote files (F3/F4/Enter on a remote entry) being watched
    /// for local edits to push back — populated by `remote::absorb_download`,
    /// consumed by Task 11's auto-upload.
    pub(crate) watches: Vec<remote::Watch>,
}

/// The session-wide `$PATH` binaries cache backing [`FarPane::bins`]: every
/// pane clones the same `Arc`, so whichever pane's background scan finishes
/// first fills it for all of them, and at most one scan actually needs to
/// run per session (see the `bins` field doc).
fn shared_bins() -> std::sync::Arc<std::sync::OnceLock<Vec<String>>> {
    static BINS: std::sync::OnceLock<std::sync::Arc<std::sync::OnceLock<Vec<String>>>> =
        std::sync::OnceLock::new();
    BINS.get_or_init(|| std::sync::Arc::new(std::sync::OnceLock::new()))
        .clone()
}

impl FarPane {
    /// Open both panels on `cwd`.
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            left: Panel::new(cwd.clone()),
            right: Panel::new(cwd),
            active: Side::Left,
            prompt: None,
            cmdline: String::new(),
            running: None,
            history: cmdhist::CmdHistory::load(),
            complete: None,
            bins: shared_bins(),
            bins_scan_started: false,
            ask: None,
            pending: None,
            drive_select: None,
            watches: Vec::new(),
        }
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::render(self, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent, alt: bool) -> Option<FarAction> {
        keys::reduce(self, key, alt)
    }

    /// Scroll the active panel by moving its cursor; `render` follows it.
    /// Positive `lines` moves toward the top of the listing.
    pub fn scroll(&mut self, lines: i32) {
        let p = self.active_panel_mut();
        let len = p.entries.len() as i64;
        if len == 0 {
            return;
        }
        p.sel = (p.sel as i64 - lines as i64).clamp(0, len - 1) as usize;
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
