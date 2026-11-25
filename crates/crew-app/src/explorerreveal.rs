//! The Explorer follows the file in front of you — VS Code's auto-reveal.
//!
//! The tree marked a file only when it was opened from the tree. A viewer opened any other way —
//! Cmd+P, a clicked link, `/view path` — left the tree closed above it and the file unmarked,
//! so "where is this?" meant walking the folders by hand. Now, while the Explorer is the open
//! section, a focused viewer whose file is under one of the tree's roots (the pane's, then each
//! pinned one) opens every folder down to it,
//! marks it, and scrolls its row to the middle of the panel when it was off screen.
//!
//! **Once per change.** It runs when the focused pane or its file changes (or the Explorer
//! opens), not every frame: a tree the user scrolls after the reveal stays where they put it
//! until they look at something else. A file outside every root leaves the tree alone.
use std::path::{Path, PathBuf};

use crate::app::CrewApp;
use crate::navsection::NavSection;

impl CrewApp {
    /// Reveal the focused viewer's file in the tree, when it changed since the last reveal.
    /// Called every frame (`build_frame`) and by Cmd+Shift+E; costs a comparison when nothing
    /// changed.
    pub(crate) fn explorer_reveal(&mut self) {
        let shown = self.config.show_nav
            && !self.config.nav_collapsed
            && self.nav_section() == NavSection::Explorer;
        if !shown {
            // Forgotten, so opening the Explorer reveals what is in front of you then.
            self.sidebar.explorer.revealed = None;
            return;
        }
        let Some(pane) = self.panes.get(self.focused) else {
            return;
        };
        let file = match &pane.content {
            crate::pane::PaneContent::View(v) => Some(v.path.as_path()),
            _ => None,
        };
        // Compared by reference: this runs every frame the Explorer shows.
        let seen = self.sidebar.explorer.revealed.as_ref();
        if seen.is_some_and(|(b, f)| *b == pane.born_ms && f.as_deref() == file) {
            return;
        }
        let file = file.map(Path::to_path_buf);
        self.sidebar.explorer.revealed = Some((pane.born_ms, file.clone()));
        let Some(file) = file else {
            return;
        };
        // The first root the file is under: the pane's, then each pinned one.
        let found = (self.explorer_roots().into_iter()).find_map(|r| Some((under(&r, &file)?, r)));
        let Some((rel, root)) = found else {
            return;
        };
        let e = &mut self.sidebar.explorer;
        e.shut.remove(&root);
        let mut dir = root.clone();
        let mut parts = rel.components().peekable();
        while let Some(c) = parts.next() {
            if parts.peek().is_some() {
                dir.push(c);
                e.open.insert(dir.clone());
            }
        }
        let path = root.join(&rel);
        e.picked = Some(path.clone());
        let items = self.explorer_items();
        let row = items.iter().position(|i| i.path == path);
        // A file the tree does not list (hidden, past the cap, written since the folder was
        // read) is marked for when it shows, but there is no row to scroll to.
        if let Some(row) = row {
            self.explorer_show_row(row, Some(items.len()));
        }
    }
}

/// `file` relative to `root`, as written or — macOS's temp and home paths run through
/// symlinks — once both are resolved.
fn under(root: &Path, file: &Path) -> Option<PathBuf> {
    // `/view ../crew/x.rs` keeps its `..`: a raw prefix match would pass `<root>/../other`.
    let (root, file) = (
        &crate::pathexpand::normalized(root),
        &crate::pathexpand::normalized(file),
    );
    if let Ok(rel) = file.strip_prefix(root) {
        return Some(rel.to_path_buf()).filter(|r| !r.as_os_str().is_empty());
    }
    let (root, file) = (root.canonicalize().ok()?, file.canonicalize().ok()?);
    let rel = file.strip_prefix(root).ok()?;
    Some(rel.to_path_buf()).filter(|r| !r.as_os_str().is_empty())
}

#[cfg(test)]
#[path = "explorerreveal_tests.rs"]
mod tests;
