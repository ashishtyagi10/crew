//! Key reduction for the Far pane: panel switching, cursor movement, descending
//! into directories / opening files, and closing the pane.
use std::path::PathBuf;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::{FarPane, Side};

/// A page jump (Page Up / Page Down) moves the cursor this many rows.
const PAGE: i32 = 10;

/// Outcome of a key press. `Close` tears the pane down (Esc / F10).
pub enum FarAction {
    Close,
}

pub(crate) fn reduce(p: &mut FarPane, key: &KeyEvent) -> Option<FarAction> {
    if !key.state.is_pressed() {
        return None;
    }
    match &key.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F10) => return Some(FarAction::Close),
        Key::Named(NamedKey::Tab) => {
            p.active = match p.active {
                Side::Left => Side::Right,
                Side::Right => Side::Left,
            }
        }
        Key::Named(NamedKey::ArrowDown) => move_sel(p, 1),
        Key::Named(NamedKey::ArrowUp) => move_sel(p, -1),
        Key::Named(NamedKey::PageDown) => move_sel(p, PAGE),
        Key::Named(NamedKey::PageUp) => move_sel(p, -PAGE),
        Key::Named(NamedKey::Home) => set_sel(p, 0),
        Key::Named(NamedKey::End) => set_sel(p, usize::MAX),
        Key::Named(NamedKey::Enter) => activate(p),
        Key::Named(NamedKey::Backspace) => ascend(p),
        _ => {}
    }
    None
}

/// Move the active panel's cursor by `delta`, clamped to the list.
pub(crate) fn move_sel(p: &mut FarPane, delta: i32) {
    let panel = p.active_panel_mut();
    let n = panel.entries.len();
    if n == 0 {
        return;
    }
    panel.sel = (panel.sel as i32 + delta).clamp(0, n as i32 - 1) as usize;
}

fn set_sel(p: &mut FarPane, idx: usize) {
    let panel = p.active_panel_mut();
    let n = panel.entries.len();
    if n > 0 {
        panel.sel = idx.min(n - 1);
    }
}

/// Enter the selected directory (or `..`), or open a file with the OS default.
pub(crate) fn activate(p: &mut FarPane) {
    let panel = p.active_panel_mut();
    let Some(entry) = panel.entries.get(panel.sel) else {
        return;
    };
    let (is_parent, is_dir, name) = (entry.is_parent, entry.is_dir, entry.name.clone());
    if is_parent {
        ascend(p);
    } else if is_dir {
        panel.cwd.push(name);
        panel.sel = 0;
        panel.reload();
    } else {
        let _ = open::that(panel.cwd.join(name));
    }
}

/// Move the active panel up to its parent directory.
pub(crate) fn ascend(p: &mut FarPane) {
    let panel = p.active_panel_mut();
    if let Some(parent) = panel.cwd.parent().map(PathBuf::from) {
        panel.cwd = parent;
        panel.sel = 0;
        panel.reload();
    }
}
