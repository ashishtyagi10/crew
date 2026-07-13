//! Session restore: quitting snapshots each terminal pane's shell working
//! directory to `session.toml` beside the config; `/restore` reopens one
//! shell per saved directory. Pull-based on purpose — startup keeps the
//! welcome state, and the snapshot is only spent when the user asks.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::app::CrewApp;
use crate::pane::PaneContent;

/// Most directories a snapshot keeps (and `/restore` reopens) — matches the
/// grid's full-tile cap so a restore never lands panes straight in the
/// minimized strip.
const MAX_DIRS: usize = 6;

#[derive(Serialize, Deserialize, Default)]
struct Session {
    dirs: Vec<String>,
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("session.toml"))
}

/// Persist `dirs` (deduped in order, capped). An empty list deletes the file
/// so a pane-less quit never offers a stale restore.
fn save_at(p: Option<PathBuf>, dirs: Vec<String>) {
    let Some(p) = p else { return };
    let mut seen = std::collections::HashSet::new();
    let dirs: Vec<String> = dirs
        .into_iter()
        .filter(|d| seen.insert(d.clone()))
        .take(MAX_DIRS)
        .collect();
    if dirs.is_empty() {
        let _ = std::fs::remove_file(&p);
        return;
    }
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = toml::to_string(&Session { dirs }) {
        let _ = std::fs::write(&p, s);
    }
}

/// The saved directories that still exist as directories.
fn load_at(p: Option<PathBuf>) -> Vec<String> {
    let Some(p) = p else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(&p) else {
        return Vec::new();
    };
    toml::from_str::<Session>(&text)
        .map(|s| s.dirs)
        .unwrap_or_default()
        .into_iter()
        .filter(|d| std::path::Path::new(d).is_dir())
        .collect()
}

impl CrewApp {
    /// Snapshot every terminal pane's shell cwd (hidden panes included —
    /// they are live shells). Asks the OS for each shell's *current*
    /// directory (the user cd's around); falls back to the pane's spawn dir
    /// where that isn't available (e.g. Windows).
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
        let dirs = self
            .panes
            .iter()
            .filter_map(|p| {
                let PaneContent::Terminal(t) = &p.content else {
                    return None;
                };
                t.pty
                    .shell_pid()
                    .and_then(|pid| sys.process(Pid::from_u32(pid)))
                    .and_then(|proc| proc.cwd())
                    .map(|c| c.to_path_buf())
                    .or_else(|| p.dir.clone())
                    .map(|d| d.to_string_lossy().into_owned())
            })
            .collect();
        save_at(path(), dirs);
    }

    /// `/restore` — reopen one shell per saved directory.
    pub(crate) fn restore_session(&mut self) {
        let dirs = load_at(path());
        self.restore_from(dirs);
    }

    /// Spawn a shell per dir by steering the tracked cwd through the normal
    /// `spawn_new_pane` path (grid sizing, notify patterns, focus, error
    /// status all included), then restoring it.
    pub(crate) fn restore_from(&mut self, dirs: Vec<String>) {
        if dirs.is_empty() {
            self.set_status("no saved session to restore".to_string());
            return;
        }
        let n = dirs.len();
        let kept = std::mem::take(&mut self.cwd);
        for d in dirs {
            self.cwd = PathBuf::from(d);
            self.spawn_new_pane();
        }
        self.cwd = kept;
        self.set_status(format!(
            "restored {n} shell{}",
            if n == 1 { "" } else { "s" }
        ));
    }
}

#[cfg(test)]
#[path = "sessionsave_tests.rs"]
mod tests;
