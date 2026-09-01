//! `/disk` — where the space went, as a treemap.
//!
//! `du | sort -n` answers the question in a column of numbers you have to
//! compare by reading. The same directory as areas answers it in one look:
//! the tile that is half the pane IS half the disk.
//!
//! The walk runs on a worker thread and reports partial totals as it goes
//! (see [`crate::project-winit-mainthread-blocking`] rationale: everything in
//! this app runs on the winit thread, so a `read_dir` over a large tree here
//! would freeze every pane). The map redraws while the scan is still running,
//! so a big tree fills in rather than making you wait for a blank pane.
pub(crate) use crate::disktile::*;
pub(crate) use crate::diskwalk::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// One child of the scanned directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Child {
    pub name: String,
    pub bytes: u64,
    pub is_dir: bool,
}

/// What the worker fills in as it walks.
#[derive(Default)]
pub(crate) struct Scan {
    pub(crate) children: Mutex<Vec<Child>>,
    /// Files visited so far — the progress readout.
    pub(crate) seen: AtomicU64,
    pub(crate) done: AtomicBool,
    pub(crate) cancel: AtomicBool,
}

pub struct DiskPane {
    pub(crate) root: PathBuf,
    pub(crate) scan: Arc<Scan>,
    /// The last snapshot taken off the worker, sorted descending.
    pub(crate) children: Vec<Child>,
    pub(crate) total: u64,
    pub(crate) scanning: bool,
    pub(crate) files: u64,
    /// The tile the keyboard is on, if any.
    pub(crate) selected: usize,
}

impl DiskPane {
    pub fn new(root: PathBuf) -> Self {
        let mut p = Self {
            root,
            scan: Arc::new(Scan::default()),
            children: Vec::new(),
            total: 0,
            scanning: false,
            files: 0,
            selected: 0,
        };
        p.start();
        p
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Descend into the selected tile (a directory), or go up with `None`.
    pub fn open(&mut self, child: Option<&str>) {
        match child {
            Some(name) => self.root.push(name),
            None => {
                if !self.root.pop() {
                    return;
                }
            }
        }
        self.start();
    }
}

impl crate::app::CrewApp {
    /// A click on a disk map picks the tile under the pointer — and picks it
    /// *again* to open it, which is how a map is used: point at the big thing,
    /// then go in. Returns true when the click was ours.
    pub(crate) fn disk_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        if !matches!(self.panes[i].content, crate::pane::PaneContent::Disk(_)) {
            return false;
        }
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let grid = self.panes[i].grid;
        let crate::pane::PaneContent::Disk(d) = &mut self.panes[i].content else {
            return false;
        };
        self.focused = i;
        self.input.focused = false;
        let Some(hit) = d.tile_at(col as f32, row as f32, grid.cols, grid.rows) else {
            // A click on the map's empty margin still focuses the pane; it
            // just does not move the selection.
            return true;
        };
        if hit == d.selected {
            if let Some(name) = d.child(hit).filter(|c| c.is_dir).map(|c| c.name.clone()) {
                d.open(Some(&name));
            }
        } else {
            d.selected = hit;
        }
        true
    }
}

impl DiskPane {
    /// The tile under a cell coordinate, if any.
    pub fn tile_at(&self, col: f32, row: f32, cols: u16, rows: u16) -> Option<usize> {
        tiles(&self.children, cols, rows)
            .into_iter()
            .find(|t| t.contains(col, row))
            .map(|t| t.index)
    }
}

#[cfg(test)]
impl DiskPane {}

impl Drop for DiskPane {
    /// A closed pane must not keep walking someone's home directory.
    fn drop(&mut self) {
        self.scan.cancel.store(true, Ordering::Relaxed);
    }
}

impl DiskPane {}

/// What a key press asked the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskAction {
    Close,
    Redraw,
}

impl DiskPane {
    /// The map's keys: pick a tile, open it, go up, rescan, close.
    ///
    /// Left/Right walk the tiles in size order rather than by position — the
    /// order the list is already in, and the order you care about: the next
    /// key takes you to the next biggest thing.
    pub fn on_key(&mut self, event: &winit::event::KeyEvent) -> Option<DiskAction> {
        use winit::keyboard::{Key, NamedKey};
        if !event.state.is_pressed() {
            return None;
        }
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => Some(DiskAction::Close),
            Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowDown) => {
                if self.children.is_empty() {
                    return None;
                }
                self.selected = (self.selected + 1) % self.children.len();
                Some(DiskAction::Redraw)
            }
            Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowUp) => {
                if self.children.is_empty() {
                    return None;
                }
                self.selected = self
                    .selected
                    .checked_sub(1)
                    .unwrap_or(self.children.len() - 1);
                Some(DiskAction::Redraw)
            }
            Key::Named(NamedKey::Enter) => {
                // Only a directory can be descended into; a file tile is the
                // end of the road and says so by not moving.
                let name = self.child(self.selected).filter(|c| c.is_dir)?.name.clone();
                self.open(Some(&name));
                Some(DiskAction::Redraw)
            }
            Key::Named(NamedKey::Backspace) => {
                self.open(None);
                Some(DiskAction::Redraw)
            }
            Key::Character(c) if c.as_str() == "r" => {
                self.rescan();
                Some(DiskAction::Redraw)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "diskpane_tests.rs"]
mod tests;
