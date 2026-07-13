//! Session snapshot persistence: quitting records each restorable pane
//! (shell, Far, /crew chat) to `session.toml` beside the config; `/restore`
//! (see `sessionrestore`) reopens them. Pull-based on purpose — startup
//! keeps the welcome state, and the snapshot is only spent when asked.
//!
//! Format v2 is a `[[panes]]` list of `kind` (+ `dir` where it applies);
//! v1 files (a bare `dirs` array of shell cwds, v0.5.73–74) still load.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Most panes a snapshot keeps (and `/restore` reopens) — matches the
/// grid's full-tile cap so a restore never lands panes straight in the
/// minimized strip.
pub(crate) const MAX_PANES: usize = 6;

/// One restorable pane. `kind` is an open string so a newer file read by an
/// older build skips unknown kinds instead of failing the whole load.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct SavedPane {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// Minimized into the left nav when saved — restored the same way.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub min: bool,
}

impl SavedPane {
    pub(crate) fn shell(dir: String) -> Self {
        SavedPane {
            kind: "shell".into(),
            dir: Some(dir),
            min: false,
        }
    }
    pub(crate) fn far(dir: String) -> Self {
        SavedPane {
            kind: "far".into(),
            dir: Some(dir),
            min: false,
        }
    }
    pub(crate) fn crew() -> Self {
        SavedPane {
            kind: "crew".into(),
            dir: None,
            min: false,
        }
    }

    /// Valid to restore: known kind, and dir-backed kinds still have their
    /// directory.
    fn restorable(&self) -> bool {
        match self.kind.as_str() {
            "shell" | "far" => self
                .dir
                .as_deref()
                .is_some_and(|d| std::path::Path::new(d).is_dir()),
            "crew" => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Session {
    /// v1 (v0.5.73–74): bare shell cwds. Read-only compat — never written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dirs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    panes: Vec<SavedPane>,
}

/// `session.toml` beside the config file.
pub(crate) fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("session.toml"))
}

/// Persist `panes` (deduped in order, capped). An empty list deletes the
/// file so a pane-less quit never offers a stale restore.
pub(crate) fn save_at(p: Option<PathBuf>, panes: Vec<SavedPane>) {
    let Some(p) = p else { return };
    let mut seen = std::collections::HashSet::new();
    let panes: Vec<SavedPane> = panes
        .into_iter()
        .filter(|sp| seen.insert((sp.kind.clone(), sp.dir.clone(), sp.min)))
        .take(MAX_PANES)
        .collect();
    if panes.is_empty() {
        let _ = std::fs::remove_file(&p);
        return;
    }
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = toml::to_string(&Session {
        dirs: Vec::new(),
        panes,
    }) {
        let _ = std::fs::write(&p, s);
    }
}

/// The saved panes that are still restorable — deduped and capped HERE too,
/// not just on save: the file is user-editable, and a hostile/fat one must
/// not fork dozens of processes on the winit thread.
pub(crate) fn load_at(p: Option<PathBuf>) -> Vec<SavedPane> {
    let Some(p) = p else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(&p) else {
        return Vec::new();
    };
    let s: Session = toml::from_str(&text).unwrap_or_default();
    let panes = if s.panes.is_empty() {
        s.dirs.into_iter().map(SavedPane::shell).collect()
    } else {
        s.panes
    };
    let mut seen = std::collections::HashSet::new();
    panes
        .into_iter()
        .filter(SavedPane::restorable)
        .filter(|sp| seen.insert((sp.kind.clone(), sp.dir.clone(), sp.min)))
        .take(MAX_PANES)
        .collect()
}

/// How many panes the current snapshot would reopen — drives the welcome
/// screen's `/restore` hint. One small file read; called once at startup.
pub(crate) fn saved_count() -> usize {
    load_at(path()).len()
}

#[cfg(test)]
#[path = "sessionsave_tests.rs"]
mod tests;
