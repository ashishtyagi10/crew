//! The app side of session restore: `save_session` snapshots the restorable
//! panes at quit (`handler::exiting`), `/restore` replays the snapshot.
//! Persistence format + file I/O live in `sessionsave`.
use std::path::PathBuf;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::sessionsave::{load_at, path, save_at, SavedPane};

impl CrewApp {
    /// Snapshot every restorable pane, hidden ones included (they're live):
    /// shells save the OS-reported *current* directory of the shell process
    /// (the user cd's around; spawn dir is the Windows/dead-shell fallback),
    /// Far panes their active panel's directory, the `/crew` chat pane its
    /// presence.
    pub(crate) fn save_session(&self) {
        let pids: Vec<Pid> = self
            .panes
            .iter()
            .filter_map(|p| match &p.content {
                PaneContent::Terminal(t) => t.pty.shell_pid().map(Pid::from_u32),
                _ => None,
            })
            .collect();
        let mut sys = System::new();
        if !pids.is_empty() {
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&pids),
                false,
                ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
            );
        }
        let panes = self
            .panes
            .iter()
            .filter_map(|p| match &p.content {
                PaneContent::Terminal(t) => t
                    .pty
                    .shell_pid()
                    .and_then(|pid| sys.process(Pid::from_u32(pid)))
                    .and_then(|proc| proc.cwd())
                    .map(|c| c.to_path_buf())
                    .or_else(|| p.dir.clone())
                    .map(|d| SavedPane::shell(d.to_string_lossy().into_owned())),
                PaneContent::Far(f) => Some(SavedPane::far(
                    f.active_cwd().to_string_lossy().into_owned(),
                )),
                PaneContent::Chat(_) if p.label.as_deref() == Some("crew") => {
                    Some(SavedPane::crew())
                }
                _ => None,
            })
            .collect::<Vec<SavedPane>>();
        // Overwrite (or, when empty, delete) the snapshot only when this
        // session actually ran restorable panes. Otherwise a welcome-screen
        // quit or a GPU-init failure exit would wipe the very snapshot
        // /restore exists to keep.
        if !panes.is_empty() || self.had_restorable {
            save_at(path(), panes);
        }
    }

    /// `/restore` — reopen the saved panes, consuming the snapshot (so a
    /// second `/restore` can't double the panes; the next quit re-saves
    /// from the live panes anyway).
    pub(crate) fn restore_session(&mut self) {
        let panes = load_at(path());
        if !panes.is_empty() {
            if let Some(p) = path() {
                let _ = std::fs::remove_file(p);
            }
        }
        self.restore_hint = None;
        self.restore_from(panes);
    }

    /// Reopen each saved pane through its normal spawn path (grid sizing,
    /// notify patterns, focus, error status all included) — shells and Far
    /// panes by steering the tracked cwd, `/crew` by its own spawner.
    pub(crate) fn restore_from(&mut self, panes: Vec<SavedPane>) {
        if panes.is_empty() {
            self.set_status("no saved session to restore".to_string());
            return;
        }
        let n = panes.len();
        let before = self.panes.len();
        let kept = std::mem::take(&mut self.cwd);
        for sp in panes {
            if let Some(d) = &sp.dir {
                self.cwd = PathBuf::from(d);
            }
            match sp.kind.as_str() {
                "shell" => self.spawn_new_pane(),
                "far" => self.spawn_far_pane(),
                "crew" => self.spawn_crew_pane(),
                _ => {} // load_at filters unknown kinds; belt for callers
            }
        }
        self.cwd = kept;
        // Count what actually opened — the spawners report failures via
        // set_status, and a blanket "restored n" would overwrite the error
        // with a lie.
        let opened = self.panes.len() - before;
        if opened == n {
            self.set_status(format!(
                "restored {n} pane{}",
                if n == 1 { "" } else { "s" }
            ));
        } else if opened > 0 {
            self.set_status(format!("restored {opened} of {n} panes"));
        }
    }
}

#[cfg(test)]
#[path = "sessionrestore_tests.rs"]
mod tests;
