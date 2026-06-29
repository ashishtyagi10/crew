//! Key reduction for the Far pane: panel switching, cursor movement, descending
//! into directories / opening files, the classic function-key actions
//! (copy/move/delete/make-folder/view/edit/help), and closing the pane.
use std::path::{Path, PathBuf};

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::{FarPane, Prompt};

/// A page jump (Page Up / Page Down) moves the cursor this many rows.
const PAGE: i32 = 10;

/// Outcome of a key press the host app must act on. Filesystem mutations happen
/// in-place on the pane; these are the effects that need the wider app.
pub enum FarAction {
    /// Tear the pane down (Esc / F10).
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
    match &key.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F10) => return Some(FarAction::Close),
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
        Key::Named(NamedKey::Enter) => return activate(p),
        Key::Named(NamedKey::Backspace) => ascend(p),
        // F3 View / F4 Edit both open the selected file with the OS default app.
        Key::Named(NamedKey::F3) | Key::Named(NamedKey::F4) => return open_selected(p),
        Key::Named(NamedKey::F5) => return Some(copy(p)),
        Key::Named(NamedKey::F6) => return Some(rename_move(p)),
        Key::Named(NamedKey::F7) => p.prompt = Some(Prompt::mkdir()),
        Key::Named(NamedKey::F8) => return Some(delete(p)),
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

/// F5: copy the active panel's selection into the other panel's directory.
pub(crate) fn copy(p: &mut FarPane) -> FarAction {
    let Some((name, is_dir, src, dst)) = transfer_paths(p) else {
        return FarAction::Status("nothing to copy".into());
    };
    if dst.exists() {
        return FarAction::Status(format!("‘{name}’ already exists in the other panel"));
    }
    let res = if is_dir {
        copy_dir_all(&src, &dst)
    } else {
        std::fs::copy(&src, &dst).map(|_| ())
    };
    match res {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("copied ‘{name}’"))
        }
        Err(e) => FarAction::Status(format!("copy failed: {e}")),
    }
}

/// F6: move (rename) the active panel's selection into the other panel's dir.
pub(crate) fn rename_move(p: &mut FarPane) -> FarAction {
    let Some((name, is_dir, src, dst)) = transfer_paths(p) else {
        return FarAction::Status("nothing to move".into());
    };
    if dst.exists() {
        return FarAction::Status(format!("‘{name}’ already exists in the other panel"));
    }
    // `rename` is atomic on the same filesystem; fall back to copy-then-remove
    // across mounts (where it fails with EXDEV).
    let res = std::fs::rename(&src, &dst).or_else(|_| {
        let copied = if is_dir {
            copy_dir_all(&src, &dst)
        } else {
            std::fs::copy(&src, &dst).map(|_| ())
        };
        copied.and_then(|_| {
            if is_dir {
                std::fs::remove_dir_all(&src)
            } else {
                std::fs::remove_file(&src)
            }
        })
    });
    match res {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("moved ‘{name}’"))
        }
        Err(e) => FarAction::Status(format!("move failed: {e}")),
    }
}

/// F8: send the active panel's selection to the OS trash (recoverable).
pub(crate) fn delete(p: &mut FarPane) -> FarAction {
    let panel = p.panel(p.active);
    let Some(entry) = panel.entries.get(panel.sel) else {
        return FarAction::Status("nothing to delete".into());
    };
    if entry.is_parent {
        return FarAction::Status("can't delete the ‘..’ entry".into());
    }
    let name = entry.name.clone();
    let path = panel.cwd.join(&name);
    match trash::delete(&path) {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("deleted ‘{name}’ to trash"))
        }
        Err(e) => FarAction::Status(format!("delete failed: {e}")),
    }
}

/// F7 (on confirm): create `name` as a directory in the active panel.
pub(crate) fn make_dir(p: &mut FarPane, name: &str) -> FarAction {
    let dir = p.panel(p.active).cwd.join(name);
    if dir.exists() {
        return FarAction::Status(format!("‘{name}’ already exists"));
    }
    match std::fs::create_dir(&dir) {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("created ‘{name}/’"))
        }
        Err(e) => FarAction::Status(format!("mkdir failed: {e}")),
    }
}

/// `(name, is_dir, source, destination)` for a copy/move of the active panel's
/// selection into the other panel's directory, or `None` when there's nothing
/// transferable selected (empty list or the `..` row).
fn transfer_paths(p: &FarPane) -> Option<(String, bool, PathBuf, PathBuf)> {
    let src_panel = p.panel(p.active);
    let entry = src_panel.entries.get(src_panel.sel)?;
    if entry.is_parent {
        return None;
    }
    let name = entry.name.clone();
    let src = src_panel.cwd.join(&name);
    let dst = p.panel(p.other_side()).cwd.join(&name);
    Some((name, entry.is_dir, src, dst))
}

/// Recursively copy directory `src` to `dst` (std has no built-in for this).
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
