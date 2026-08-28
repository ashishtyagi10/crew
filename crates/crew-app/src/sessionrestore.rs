//! The app side of session restore: `save_session` snapshots the restorable
//! panes at quit (`handler::exiting`), `/restore` replays the snapshot.
//! Persistence format + file I/O live in `sessionsave`.
use std::path::{Path, PathBuf};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::sessionsave::{load_at, path, save_at, SavedPane};

/// The tracked cwd to restore `sp` into for its spawn iteration: a
/// dir-backed entry's own directory, or `kept` for anything whose `dir`
/// isn't a directory at all — a remote Far pane (`dir` is an rclone
/// address) or a view pane (`dir` is a *file*, not somewhere a shell/Far
/// spawn could `cd` into). Pulled out of `restore_from` so the rule is
/// unit-testable without going through a full restore.
pub(crate) fn restore_cwd_for(sp: &SavedPane, kept: &Path) -> PathBuf {
    let is_remote_far = sp.kind == "far" && sp.remote;
    if is_remote_far || sp.kind == "view" {
        return kept.to_path_buf();
    }
    sp.dir
        .as_deref()
        .map_or_else(|| kept.to_path_buf(), PathBuf::from)
}

impl CrewApp {
    /// Snapshot every restorable pane, hidden ones included (they're live):
    /// shells save the OS-reported *current* directory of the shell process
    /// (the user cd's around; spawn dir is the Windows/dead-shell fallback),
    /// Far panes their active panel's location (a local dir, or an rclone
    /// `remote:path` address when the active panel is browsing a remote),
    /// the `/crew` chat pane (routing label "crew") its presence, a file
    /// viewer the full resolved path it has open.
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
                    PaneContent::Far(f) => {
                        let loc = f.active_loc();
                        if loc.is_remote() {
                            Some(SavedPane::far_remote(loc.rclone_addr()))
                        } else {
                            loc.local_path()
                                .map(|p| SavedPane::far(p.to_string_lossy().into_owned()))
                        }
                    }
                    PaneContent::Chat(_) if p.label.as_deref() == Some("crew") => {
                        Some(SavedPane::crew())
                    }
                    // Fix 4: `/about` and `??` open their viewer on a
                    // SYNTHETIC temp file (a changelog, an explanation) —
                    // not something the user asked to view, and one whose
                    // path won't mean anything on the next launch anyway.
                    // Saving it would let a run whose only pane is `/about`
                    // silently replace a saved multi-shell session with a
                    // changelog viewer.
                    PaneContent::View(v) if !v.ephemeral => {
                        Some(SavedPane::view(v.path.to_string_lossy().into_owned()))
                    }
                    PaneContent::Todo(_) => Some(SavedPane::todo()),
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

    /// Reopen ONE saved pane through its normal spawn path (grid sizing,
    /// notify patterns, focus, error status all included) — shells and Far
    /// panes by steering the tracked cwd, `/crew` by its own spawner. A
    /// remote Far pane also gets its active panel re-rooted onto the saved
    /// `remote:path` address, and its listing kicked off, right after spawn.
    ///
    /// `kept` is the cwd to fall back on for an entry that names no directory
    /// of its own; `self.cwd` is steered to the spawn directory and left
    /// there, so a caller replaying several entries restores it once at the
    /// end rather than after each. Returns whether a pane was actually
    /// pushed — a spawn can fail, and the caller counts what opened rather
    /// than what it asked for.
    ///
    /// Split out of `restore_from` so `/reopen` ([`crate::reopen`]) replays a
    /// single closed pane through exactly the same path a `/restore` uses.
    /// Undo-close and session restore differ only in where the entry came
    /// from; two copies of this would drift the moment a new pane kind
    /// learned to be restorable.
    pub(crate) fn open_saved(&mut self, sp: &SavedPane, kept: &Path) -> bool {
        // A remote Far pane's `dir` is an rclone address, not a local
        // path — spawn it at the tracked cwd like a dir-less entry, and
        // reconstruct the remote location below once it exists.
        let remote_addr = (sp.kind == "far" && sp.remote)
            .then(|| sp.dir.clone())
            .flatten();
        // Reset each call: a dir-less entry must spawn in the tracked cwd,
        // not leak the previous entry's directory.
        self.cwd = restore_cwd_for(sp, kept);
        let count = self.panes.len();
        match sp.kind.as_str() {
            "shell" => self.spawn_new_pane(),
            "far" => self.spawn_far_pane(),
            "crew" => self.spawn_crew_pane(),
            "todo" => self.spawn_todo_pane(),
            "usage" => self.spawn_usage_pane(),
            "disk" => self.spawn_disk_pane(None),
            "view" => {
                if let Some(path) = sp.dir.as_deref() {
                    self.open_view(path);
                }
            }
            _ => {} // load_at filters unknown kinds; belt for callers
        }
        // Re-minimize only the pane THIS call pushed (a failed spawn pushes
        // none — last_mut would hit the previous pane).
        let opened = self.panes.len() > count;
        if sp.min && opened {
            if let Some(p) = self.panes.last_mut() {
                p.hidden = true;
            }
        }
        if let Some(addr) = remote_addr {
            if let Some(pane) = self.panes.last_mut() {
                if let PaneContent::Far(f) = &mut pane.content {
                    let _ = f.restore_remote(&addr);
                }
            }
        }
        opened
    }

    /// Reopen each saved pane through its normal spawn path (grid sizing,
    /// notify patterns, focus, error status all included) — shells and Far
    /// panes by steering the tracked cwd, `/crew` by its own spawner. A
    /// remote Far pane also gets its active panel re-rooted onto the saved
    /// `remote:path` address, and its listing kicked off, right after spawn.
    pub(crate) fn restore_from(&mut self, panes: Vec<SavedPane>) {
        if panes.is_empty() {
            self.set_status("no saved session to restore".to_string());
            return;
        }
        let n = panes.len();
        // `open_view` (called below, for a "view" entry) always sets
        // `self.zoomed = true` — the right call for a fresh `/view`, and for
        // restoring a session that is nothing BUT a viewer. It is wrong for
        // any other restored set: `[view, shell, shell, shell]` used to leave
        // `zoomed == true` with focus on the last shell, so the user saw one
        // pane and had every reason to think the other three failed to open.
        // Zooming only when the WHOLE restored set is a single viewer keeps
        // the single-pane case (still exactly what a fresh `/view` would do)
        // and clears it for every other shape, rather than unconditionally
        // clearing `zoomed` at the end — which would also un-zoom the
        // single-viewer restore a user very much wants zoomed.
        let single_view = n == 1 && panes.first().is_some_and(|sp| sp.kind == "view");
        let before = self.panes.len();
        let kept = std::mem::take(&mut self.cwd);
        for sp in panes {
            self.open_saved(&sp, &kept);
        }
        self.cwd = kept;
        // Fix 3: only a single-viewer restore should stay zoomed — see the
        // comment on `single_view` above.
        if !single_view {
            self.zoomed = false;
        }
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
