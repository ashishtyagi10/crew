//! Far Manager pane: a dual-pane file browser (two side-by-side directory
//! listings) spawned by `/far`. Tab switches the active panel; arrows move the
//! cursor; Enter descends into a folder (or `..`) or opens a file with the OS
//! default. The function-key bar works as labelled: F1 help, F3/F4 view/edit
//! (open with the OS default), F5 copy and F6 move into the other panel, F7
//! make-folder (a text prompt), F8 delete to trash, F10/Esc close. Lives in the
//! auto-tiling grid like any other pane and renders into a `ratatui` buffer →
//! GPU cells.
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

/// An in-pane single-line text prompt — currently only "make folder" (F7).
pub(crate) struct Prompt {
    pub kind: PromptKind,
    pub input: String,
}

#[derive(Clone, Copy)]
pub(crate) enum PromptKind {
    MkDir,
}

impl Prompt {
    pub(crate) fn mkdir() -> Self {
        Self {
            kind: PromptKind::MkDir,
            input: String::new(),
        }
    }
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
    /// Active text prompt (F7 make-folder), captured before any nav key.
    pub(crate) prompt: Option<Prompt>,
}

impl FarPane {
    /// Open both panels on `cwd`.
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            left: Panel::new(cwd.clone()),
            right: Panel::new(cwd),
            active: Side::Left,
            prompt: None,
        }
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::render(self, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent) -> Option<FarAction> {
        keys::reduce(self, key)
    }

    /// Scroll the active panel by moving its cursor; `render` follows it.
    /// Positive `lines` moves toward the top of the listing.
    pub fn scroll(&mut self, lines: i32) {
        let p = self.active_panel_mut();
        let len = p.entries.len() as i64;
        if len == 0 {
            return;
        }
        p.sel = (p.sel as i64 - lines as i64).clamp(0, len - 1) as usize;
    }

    pub(crate) fn active_panel_mut(&mut self) -> &mut Panel {
        self.panel_mut(self.active)
    }

    /// The panel on the side *opposite* the active one — the destination for
    /// copy/move operations.
    pub(crate) fn other_side(&self) -> Side {
        match self.active {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub(crate) fn panel(&self, side: Side) -> &Panel {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    pub(crate) fn panel_mut(&mut self, side: Side) -> &mut Panel {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    /// Re-read both panels after a filesystem change so each side reflects it
    /// (the two panels often show the same directory).
    pub(crate) fn reload_both(&mut self) {
        self.left.reload();
        self.right.reload();
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
