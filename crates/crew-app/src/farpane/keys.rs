//! Key reduction for the Far pane: panel switching, cursor movement, descending
//! into directories / opening files, the classic function-key actions
//! (copy/move/delete/make-folder/view/edit/help), and closing the pane.
use std::path::PathBuf;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::fileops::{copy, delete, make_dir, rename_move};
use super::run::run_cmdline;
use super::{FarPane, Prompt};

/// A page jump (Page Up / Page Down) moves the cursor this many rows.
const PAGE: i32 = 10;

/// Outcome of a key press the host app must act on. Filesystem mutations happen
/// in-place on the pane; these are the effects that need the wider app.
pub enum FarAction {
    /// Tear the pane down (Esc on an empty command line / F10).
    Close,
    /// Open the keyboard-shortcuts overlay (F1).
    Help,
    /// Open a file with the OS default application (F3 / F4 / Enter on a file).
    Open(PathBuf),
    /// Show a transient status message (operation result or error).
    Status(String),
}

pub(crate) fn reduce(p: &mut FarPane, key: &KeyEvent) -> Option<FarAction> {
    if !key.state.is_pressed() {
        return None;
    }
    // A live text prompt (F7 make-folder) swallows every key until it's
    // confirmed (Enter) or cancelled (Esc).
    if p.prompt.is_some() {
        return prompt_key(p, key);
    }
    let typing = !p.cmdline.is_empty();
    match &key.logical_key {
        // F10 always quits. Esc clears a typed command first, else quits.
        Key::Named(NamedKey::F10) => return Some(FarAction::Close),
        Key::Named(NamedKey::Escape) => {
            if typing {
                p.cmdline.clear();
            } else {
                return Some(FarAction::Close);
            }
        }
        Key::Named(NamedKey::F1) => return Some(FarAction::Help),
        Key::Named(NamedKey::Tab) => {
            p.active = p.other_side();
        }
        Key::Named(NamedKey::ArrowDown) => move_sel(p, 1),
        Key::Named(NamedKey::ArrowUp) => move_sel(p, -1),
        Key::Named(NamedKey::PageDown) => move_sel(p, PAGE),
        Key::Named(NamedKey::PageUp) => move_sel(p, -PAGE),
        Key::Named(NamedKey::Home) => set_sel(p, 0),
        Key::Named(NamedKey::End) => set_sel(p, usize::MAX),
        // Enter runs a typed command; with an empty command line it activates
        // the selected entry (descend / open), preserving the old behaviour.
        Key::Named(NamedKey::Enter) => {
            if typing {
                return Some(run_cmdline(p));
            }
            return activate(p);
        }
        // Backspace edits the command line while typing, else ascends.
        Key::Named(NamedKey::Backspace) => {
            if typing {
                p.cmdline.pop();
            } else {
                ascend(p);
            }
        }
        // F3 View / F4 Edit both open the selected file with the OS default app.
        Key::Named(NamedKey::F3) | Key::Named(NamedKey::F4) => return open_selected(p),
        Key::Named(NamedKey::F5) => return Some(copy(p)),
        Key::Named(NamedKey::F6) => return Some(rename_move(p)),
        Key::Named(NamedKey::F7) => p.prompt = Some(Prompt::mkdir()),
        Key::Named(NamedKey::F8) => return Some(delete(p)),
        // Printable input builds up the command line (classic Far behaviour).
        Key::Named(NamedKey::Space) => p.cmdline.push(' '),
        Key::Character(s) => p.cmdline.push_str(s.as_str()),
        _ => {}
    }
    None
}

/// Handle a key while the make-folder prompt is open.
fn prompt_key(p: &mut FarPane, key: &KeyEvent) -> Option<FarAction> {
    match &key.logical_key {
        Key::Named(NamedKey::Escape) => {
            p.prompt = None;
            None
        }
        Key::Named(NamedKey::Enter) => {
            let name = p.prompt.take().map(|pr| pr.input).unwrap_or_default();
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(make_dir(p, name))
        }
        Key::Named(NamedKey::Backspace) => {
            if let Some(pr) = p.prompt.as_mut() {
                pr.input.pop();
            }
            None
        }
        Key::Named(NamedKey::Space) => {
            if let Some(pr) = p.prompt.as_mut() {
                pr.input.push(' ');
            }
            None
        }
        Key::Character(s) => {
            if let Some(pr) = p.prompt.as_mut() {
                pr.input.push_str(s.as_str());
            }
            None
        }
        _ => None,
    }
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

/// Enter the selected directory (or `..`), or ask the app to open a file.
pub(crate) fn activate(p: &mut FarPane) -> Option<FarAction> {
    let panel = p.active_panel_mut();
    let entry = panel.entries.get(panel.sel)?;
    let (is_parent, is_dir, name) = (entry.is_parent, entry.is_dir, entry.name.clone());
    if is_parent {
        ascend(p);
        None
    } else if is_dir {
        panel.cwd.push(name);
        panel.sel = 0;
        panel.reload();
        None
    } else {
        Some(FarAction::Open(panel.cwd.join(name)))
    }
}

/// F3/F4: open the selected file with the OS default app (directories ignored).
fn open_selected(p: &FarPane) -> Option<FarAction> {
    let panel = p.panel(p.active);
    let entry = panel.entries.get(panel.sel)?;
    if entry.is_parent || entry.is_dir {
        return None;
    }
    Some(FarAction::Open(panel.cwd.join(&entry.name)))
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
