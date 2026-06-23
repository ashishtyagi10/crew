//! Far Manager pane: a dual-pane file browser (two side-by-side directory
//! listings) spawned by `/far`. Tab switches the active panel; arrows move the
//! cursor; Enter descends into a folder (or `..`) or opens a file with the OS
//! default; Esc / F10 closes the pane. Lives in the auto-tiling grid like any
//! other pane and renders into a `ratatui` buffer → GPU cells.
mod keys;
mod list;
mod render;

use std::path::PathBuf;

use crew_render::CellView;
use winit::event::KeyEvent;

pub use keys::FarAction;

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
}

/// One side of the dual-pane manager: a directory and its sorted listing.
pub(crate) struct Panel {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub sel: usize,
}

impl Panel {
    fn new(cwd: PathBuf) -> Self {
        let entries = list::read_dir(&cwd);
        Self {
            cwd,
            entries,
            sel: 0,
        }
    }

    /// Re-read the current directory and clamp the cursor into range.
    fn reload(&mut self) {
        self.entries = list::read_dir(&self.cwd);
        self.sel = self.sel.min(self.entries.len().saturating_sub(1));
    }
}

pub struct FarPane {
    pub(crate) left: Panel,
    pub(crate) right: Panel,
    pub(crate) active: Side,
}

impl FarPane {
    /// Open both panels on `cwd`.
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            left: Panel::new(cwd.clone()),
            right: Panel::new(cwd),
            active: Side::Left,
        }
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::render(self, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent) -> Option<FarAction> {
        keys::reduce(self, key)
    }

    pub(crate) fn active_panel_mut(&mut self) -> &mut Panel {
        match self.active {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
