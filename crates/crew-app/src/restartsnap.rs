//! The restart snapshot: what `/restart` writes just before it relaunches,
//! and what the fresh process reads back to put every window where it was.
//!
//! It is not the quit snapshot (`sessionsave`), though it is made of the same
//! [`SavedPane`]s. That one is pull-based — spent only when `/restore` asks —
//! and it dedupes, because reopening yesterday's session should not hand you
//! the same shell twice. A restart is neither: it comes back by itself, and
//! two shells in one directory were two shells a second ago. So this keeps
//! every pane, in order, grouped by the window it was in, with the pane that
//! had the focus — and lives in its own file, so a restart never spends or
//! overwrites the session `/restore` is keeping.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::sessionsave::SavedPane;

/// Most panes one restart reopens, over every window. The file is
/// user-editable, and a fat one must not fork processes without bound.
const MAX_PANES: usize = 64;

/// One window's panes, in grid order.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone)]
pub(crate) struct Win {
    /// Which of `panes` had the focus; absent when the input bar did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<usize>,
    #[serde(default)]
    pub panes: Vec<SavedPane>,
}

#[derive(Serialize, Deserialize, Default)]
struct Snap {
    #[serde(default)]
    windows: Vec<Win>,
}

/// `restart.toml` beside the config file.
pub(crate) fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("restart.toml"))
}

/// Write the snapshot. False when it could not be written — the restart
/// still goes ahead, it just comes back empty.
pub(crate) fn save_at(p: Option<PathBuf>, windows: Vec<Win>) -> bool {
    let Some(p) = p else { return false };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    toml::to_string(&Snap { windows }).is_ok_and(|s| std::fs::write(&p, s).is_ok())
}

/// Throw the snapshot away — the relaunch it was written for did not happen.
pub(crate) fn discard_at(p: Option<PathBuf>) {
    if let Some(p) = p {
        let _ = std::fs::remove_file(p);
    }
}

/// Read the snapshot and consume it, so it resumes exactly one launch.
/// Panes whose directory or file has gone since are dropped (and the focus
/// index moved to match); windows left with nothing are dropped too, except
/// the first, which is the window the launch already has.
pub(crate) fn take_at(p: Option<PathBuf>) -> Vec<Win> {
    let Some(p) = p else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(&p) else {
        return Vec::new();
    };
    let _ = std::fs::remove_file(&p);
    let snap: Snap = toml::from_str(&text).unwrap_or_default();
    let mut budget = MAX_PANES;
    snap.windows
        .into_iter()
        .map(|w| keep_restorable(w, &mut budget))
        .enumerate()
        .filter(|(i, w)| *i == 0 || !w.panes.is_empty())
        .map(|(_, w)| w)
        .collect()
}

fn keep_restorable(w: Win, budget: &mut usize) -> Win {
    let mut out = Win::default();
    for (i, sp) in w.panes.into_iter().enumerate() {
        if *budget == 0 || !sp.restorable() {
            continue;
        }
        *budget -= 1;
        if w.focus == Some(i) {
            out.focus = Some(out.panes.len());
        }
        out.panes.push(sp);
    }
    out
}

#[cfg(test)]
#[path = "restartsnap_tests.rs"]
mod tests;
