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
    /// Far panes their active panel's directory, the `/crew` chat pane
    /// (routing label "crew") its presence.
    pub(crate) fn session_panes(&self) -> Vec<SavedPane> {
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
        self.panes
            .iter()
            .filter_map(|p| {
                let sp = match &p.content {
                    PaneContent::Terminal(t) => t
                        .pty
                        .shell_pid()
                        .and_then(|pid| sys.process(Pid::from_u32(pid)))
                        .and_then(|proc| proc.cwd())
                        .map(|c| c.to_path_buf())
                        .or_else(|| p.dir.clone())
                        .map(|d| SavedPane::shell(d.to_string_lossy().into_owned())),
                    PaneContent::Far(f) => f
                        .active_loc()
                        .local_path()
                        .map(|p| SavedPane::far(p.to_string_lossy().into_owned())),
                    PaneContent::Chat(_) if p.label.as_deref() == Some("crew") => {
                        Some(SavedPane::crew())
                    }
                    _ => None,
                };
                sp.map(|mut sp| {
                    sp.min = p.hidden;
                    sp
                })
            })
            .collect::<Vec<SavedPane>>()
    }

    /// Persist the snapshot at quit. Overwrite (or, when empty, delete) it
    /// only when this session actually ran restorable panes — otherwise a
    /// welcome-screen quit or a GPU-init failure exit would wipe the very
    /// snapshot /restore exists to keep.
    pub(crate) fn save_session(&self) {
        let panes = self.session_panes();
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
            // Reset each iteration: a dir-less entry must spawn in the
            // tracked cwd, not leak the previous entry's directory.
            self.cwd = sp
                .dir
                .as_deref()
                .map_or_else(|| kept.clone(), PathBuf::from);
            let count = self.panes.len();
            match sp.kind.as_str() {
                "shell" => self.spawn_new_pane(),
                "far" => self.spawn_far_pane(),
                "crew" => self.spawn_crew_pane(),
                _ => {} // load_at filters unknown kinds; belt for callers
            }
            // Re-minimize only the pane THIS iteration pushed (a failed
            // spawn pushes none — last_mut would hit the previous pane).
            if sp.min && self.panes.len() > count {
                if let Some(p) = self.panes.last_mut() {
                    p.hidden = true;
                }
            }
        }
        self.cwd = kept;
        // The loop leaves the last spawn focused; if that one restored
        // minimized, reconcile_grid's focus-restores rule would immediately
        // un-minimize it. Land focus on a visible pane instead (or the
        // input bar when everything restored minimized).
        if self.panes.get(self.focused).is_some_and(|p| p.hidden) {
            match self.nearest_visible(self.focused) {
                Some(i) => self.focused = i,
                None => self.input.focused = true,
            }
        }
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
