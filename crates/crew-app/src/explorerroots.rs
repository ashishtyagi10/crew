//! Folders pinned to the Explorer beside the focused pane's own — VS Code's multi-root
//! workspace.
//!
//! The tree showed one root, the focused pane's folder, so a credentials or properties file kept
//! somewhere else (`~/.aws`, a config repo) meant a `/view` with its whole path typed out. Now
//! `/nav explorer add <path>` pins a folder: it is drawn as a root of its own under the pane's,
//! opened and closed, revealed into, walked by the keys like it. The pane's root still follows
//! focus; the pinned ones stay put. A hovered pinned root shows a `×` that unpins it, as Delete
//! does on the keyboard's cursor (`explorerpin`).
//!
//! Persisted as `<config_dir>/crew/explorer-roots`, one path per line, in the order pinned —
//! read once, on first use, into the [`Explorer`] (not a process-wide static: tests build apps
//! side by side). Tests never touch the real file.
use std::path::{Path, PathBuf};

use crate::explorer::Explorer;

/// Most folders pinned at once.
pub(crate) const MAX: usize = 20;

/// `<config_dir>/crew/explorer-roots`. Never under test: an app built there starts with
/// nothing pinned, and pinning writes nothing — unless the test set [`TEST_PATH`].
fn path() -> Option<PathBuf> {
    #[cfg(test)]
    return TEST_PATH.with(|p| p.borrow().clone());
    #[cfg(not(test))]
    dirs::config_dir().map(|d| d.join("crew").join("explorer-roots"))
}

#[cfg(test)]
thread_local! {
    /// Where a test that proves the round trip keeps the list; `None` (every other test): nowhere.
    pub(crate) static TEST_PATH: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// Non-empty lines as paths, duplicates after the first dropped, at most [`MAX`].
pub(crate) fn parse(s: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for l in s.lines().filter(|l| !l.trim().is_empty()) {
        let p = PathBuf::from(l);
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out.truncate(MAX);
    out
}

/// The list stored at `p` (empty when there is none).
pub(crate) fn load_from(p: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(p)
        .map(|s| parse(&s))
        .unwrap_or_default()
}

/// Write `list` to `p`, one path per line.
pub(crate) fn save_to(p: &Path, list: &[PathBuf]) {
    let body: String = list.iter().map(|d| format!("{}\n", d.display())).collect();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(p, body);
}

impl Explorer {
    /// The pinned folders, in the order pinned.
    pub(crate) fn pinned(&self) -> Vec<PathBuf> {
        let mut g = self.pinned.borrow_mut();
        g.get_or_insert_with(|| path().map(|p| load_from(&p)).unwrap_or_default())
            .clone()
    }

    /// Whether `dir` is pinned.
    pub(crate) fn is_pinned(&self, dir: &Path) -> bool {
        self.pinned().iter().any(|p| p == dir)
    }

    /// Pin `dir` (already absolute and normalized) after the others. `false` when it was
    /// pinned already or the list is full.
    pub(crate) fn pin(&self, dir: &Path) -> bool {
        let mut list = self.pinned();
        if list.iter().any(|p| p == dir) || list.len() >= MAX {
            return false;
        }
        list.push(dir.to_path_buf());
        self.keep(list);
        true
    }

    /// Unpin `dir`. `true` when it was pinned.
    pub(crate) fn unpin(&self, dir: &Path) -> bool {
        let mut list = self.pinned();
        let n = list.len();
        list.retain(|p| p != dir);
        if list.len() == n {
            return false;
        }
        self.keep(list);
        true
    }

    /// Hold `list` and save it.
    fn keep(&self, list: Vec<PathBuf>) {
        if let Some(p) = path() {
            save_to(&p, &list);
        }
        *self.pinned.borrow_mut() = Some(list);
    }
}

#[cfg(test)]
#[path = "explorerroots_tests.rs"]
mod tests;
