//! Far Manager pane: a dual-pane file browser (two side-by-side directory
//! listings) spawned by `/far`. Tab switches the active panel when the command
//! line is empty; arrows move the cursor; Enter descends into a folder (or
//! `..`) or opens a file with the OS default. The function-key bar works as
//! labelled: F1 help, F3/F4 view/edit (open with the OS default), F5 copy and
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
mod ask;
mod cmdhist;
mod complete;
mod fileops;
mod icons;
mod keys;
mod list;
mod location;
mod rclone;
mod render;
mod run;

use std::path::PathBuf;

use crew_render::CellView;
use winit::event::KeyEvent;

pub use keys::FarAction;
use location::Location;

/// Which panel currently has the cursor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
}

/// One filesystem entry shown in a panel.
pub(crate) struct Entry {
    pub name: String,
    pub is_dir: bool,
    /// The synthetic ".." row that ascends to the parent directory.
    pub is_parent: bool,
    /// File size in bytes; 0 for directories and the parent row.
    pub size: u64,
}

/// An in-pane single-line text prompt — currently only "make folder" (F7).
pub(crate) struct Prompt {
    pub kind: PromptKind,
    pub input: String,
}

#[derive(Clone, Copy)]
pub(crate) enum PromptKind {
    MkDir,
}

impl Prompt {
    pub(crate) fn mkdir() -> Self {
        Self {
            kind: PromptKind::MkDir,
            input: String::new(),
        }
    }
}

/// One side of the dual-pane manager: a location and its sorted listing.
pub(crate) struct Panel {
    pub loc: Location,
    pub entries: Vec<Entry>,
    pub sel: usize,
}

impl Panel {
    fn new(cwd: PathBuf) -> Self {
        let loc = Location::local(&cwd);
        let entries = list::read_dir(&cwd);
        Self {
            loc,
            entries,
            sel: 0,
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
        }
    }

    /// Whether a command-line command is still running or an AI ask is in
    /// flight (drives the busy sweep — which is also what repaints the
    /// `thinking… Ns` counter while waiting).
    pub fn is_busy(&self) -> bool {
        self.running.is_some() || matches!(self.ask, Some(ask::AskState::Thinking { .. }))
    }

    /// Drain the running command’s result, if it finished this tick: reload
    /// both panels (the command likely changed the directory contents) and
    /// return a status line for the app to flash.
    pub fn poll_cmd(&mut self) -> Option<String> {
        let (cmd, rx) = self.running.as_ref()?;
        let done = rx.try_recv().ok()?;
        let cmd = cmd.clone();
        self.running = None;
        self.reload_both();
        let outcome = match done.code {
            Some(0) => "ok".to_string(),
            Some(c) => format!("exit {c}"),
            None => "killed".to_string(),
        };
        Some(if done.tail.is_empty() {
            format!("‘{cmd}’ — {outcome}")
        } else {
            format!("‘{cmd}’ — {outcome} · {}", done.tail)
        })
    }

    /// Drain a finished `!` ask, if any: land it (via [`Self::absorb_ask_result`])
    /// or report the worker thread dying without a reply. Returns a status
    /// line for the app to flash, mirroring `poll_cmd`; `None` when nothing
    /// changed this tick (still thinking, or no ask at all).
    pub fn poll_ask(&mut self) -> Option<String> {
        let Some(ask::AskState::Thinking { rx, .. }) = &self.ask else {
            return None;
        };
        match rx.try_recv() {
            Ok(res) => Some(self.absorb_ask_result(res)),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.ask = None;
                Some("ask failed: worker died — ! text kept".to_string())
            }
        }
    }

    /// Land a finished ask’s result: a non-blank suggestion replaces
    /// `cmdline` (state becomes `Suggested`, `original` keeps the `!` text
    /// for Esc); a blank suggestion or an error clears `ask` and leaves
    /// `cmdline` untouched. Returns the status line either way.
    fn absorb_ask_result(&mut self, res: Result<String, String>) -> String {
        match res {
            Ok(cmd) if cmd.trim().is_empty() => {
                self.ask = None;
                "no command suggested — ! text kept".to_string()
            }
            Ok(cmd) => {
                let original = std::mem::replace(&mut self.cmdline, cmd.trim().to_string());
                self.ask = Some(ask::AskState::Suggested { original });
                "Enter run · Esc discard · keep typing to edit".to_string()
            }
            Err(e) => {
                self.ask = None;
                format!("ask failed: {e} — ! text kept")
            }
        }
    }

    /// The active panel's location.
    pub(crate) fn active_loc(&self) -> Location {
        self.panel(self.active).loc.clone()
    }

    /// The active panel's directory as a local path — the working dir for the
    /// bottom command line, which is LOCAL-ONLY in v1. A remote active panel
    /// yields the temp dir as an inert fallback (the command line is disabled
    /// for remote panels in `run.rs`).
    pub(crate) fn active_cwd(&self) -> PathBuf {
        self.active_loc()
            .local_path()
            .unwrap_or_else(std::env::temp_dir)
    }

    /// The active panel's directory label for the command bar: the last path
    /// segment of its display string (or the whole string when there's no
    /// separator) — mirrors the previous `cwd.file_name()` behavior for local
    /// paths, extended to remote locations via `Location::display`.
    pub(crate) fn active_panel_folder(&self) -> String {
        let display = self.active_loc().display();
        display
            .rsplit(['/', '\\'])
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or(display)
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::render(self, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent) -> Option<FarAction> {
        keys::reduce(self, key)
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

    pub(crate) fn active_panel_mut(&mut self) -> &mut Panel {
        self.panel_mut(self.active)
    }

    /// The panel on the side *opposite* the active one — the destination for
    /// copy/move operations.
    pub(crate) fn other_side(&self) -> Side {
        match self.active {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub(crate) fn panel(&self, side: Side) -> &Panel {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    pub(crate) fn panel_mut(&mut self, side: Side) -> &mut Panel {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    /// Re-read both panels after a filesystem change so each side reflects it
    /// (the two panels often show the same directory).
    pub(crate) fn reload_both(&mut self) {
        self.left.reload();
        self.right.reload();
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
