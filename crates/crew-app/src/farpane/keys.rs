//! Key reduction for the Far pane: panel switching, cursor movement, descending
//! into directories / opening files, the classic function-key actions
//! (copy/move/delete/make-folder/view/edit/help), Tab completion + Up/Down
//! history + Right/End ghost-text acceptance on the command line, and closing
//! the pane.
use std::path::PathBuf;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

pub(crate) use super::cmdline::*;
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
    /// Open a file with the OS default application: Enter on a local file, or
    /// a downloaded remote temp file (F3/F4 on a remote entry, once
    /// `begin_download` lands it — see `remote::absorb_download`).
    Open(PathBuf),
    /// F3 — show the file in the viewer pane, inside crew.
    View(PathBuf),
    /// F4 — open `$EDITOR` on the file in a terminal pane.
    Edit(PathBuf),
    /// Show a transient status message (operation result or error).
    Status(String),
}

pub(crate) fn reduce(p: &mut FarPane, key: &KeyEvent, alt: bool) -> Option<FarAction> {
    if !key.state.is_pressed() {
        return None;
    }
    // The drive-select overlay (Alt+F1/F2) swallows every key until it's
    // confirmed (Enter) or cancelled (Esc) — checked before the prompt and
    // the main match so nothing leaks through while it's open.
    if p.drive_select.is_some() {
        return drive_select_key(p, key);
    }
    // A live text prompt (F7 make-folder) swallows every key until it's
    // confirmed (Enter) or cancelled (Esc).
    if p.prompt.is_some() {
        return prompt_key(p, key);
    }
    if alt {
        match &key.logical_key {
            Key::Named(NamedKey::F1) => return Some(p.open_drive_select(super::Side::Left)),
            Key::Named(NamedKey::F2) => return Some(p.open_drive_select(super::Side::Right)),
            _ => {}
        }
    }
    let typing = !p.cmdline.is_empty();
    match &key.logical_key {
        // F10 always quits. Esc cancels a Tab-cycle, else clears a typed
        // command, else quits.
        Key::Named(NamedKey::F10) => return Some(FarAction::Close),
        Key::Named(NamedKey::Escape) => return escape_cmdline(p),
        Key::Named(NamedKey::F1) => return Some(FarAction::Help),
        // Tab stays contextual: an empty bar switches panels (unchanged); a
        // typed bar completes/cycles the caret token.
        Key::Named(NamedKey::Tab) => {
            if typing {
                tab_complete(p);
            } else {
                p.active = p.other_side();
            }
        }
        Key::Named(NamedKey::ArrowDown) => {
            if typing {
                history_next(p);
            } else {
                move_sel(p, 1);
            }
        }
        Key::Named(NamedKey::ArrowUp) => {
            if typing {
                history_prev(p);
            } else {
                move_sel(p, -1);
            }
        }
        Key::Named(NamedKey::ArrowRight) => {
            if typing {
                accept_ghost(p);
            }
        }
        Key::Named(NamedKey::PageDown) => move_sel(p, PAGE),
        Key::Named(NamedKey::PageUp) => move_sel(p, -PAGE),
        Key::Named(NamedKey::Home) => set_sel(p, 0),
        Key::Named(NamedKey::End) => {
            if typing {
                accept_ghost(p);
            } else {
                set_sel(p, usize::MAX);
            }
        }
        // Enter runs a typed command, submits a `!` ask, or (empty bar)
        // activates the selected entry (descend / open).
        Key::Named(NamedKey::Enter) => {
            if typing {
                // A landed suggestion runs verbatim via run_cmdline — even
                // when the suggested command itself starts with `!` (POSIX
                // pipeline negation), which must not re-enter the ask path.
                let suggested = matches!(p.ask, Some(super::ask::AskState::Suggested { .. }));
                if !suggested {
                    if let Some(desc) = super::ask::bang_ask(&p.cmdline) {
                        let desc = desc.to_string();
                        return Some(super::run::submit_ask(p, &desc));
                    }
                }
                return Some(run_cmdline(p));
            }
            return activate(p);
        }
        // Backspace edits the command line while typing, else ascends.
        Key::Named(NamedKey::Backspace) => {
            if typing {
                p.cmdline.pop();
                p.complete = None;
                p.ask = None;
            } else {
                return ascend(p);
            }
        }
        // F3 shows the selected file in the viewer pane; F4 opens it in
        // `$EDITOR`. Both stay inside crew — see `view_selected`/`edit_selected`.
        Key::Named(NamedKey::F3) => return view_selected(p),
        Key::Named(NamedKey::F4) => return edit_selected(p),
        Key::Named(NamedKey::F5) => return Some(copy(p)),
        Key::Named(NamedKey::F6) => return Some(rename_move(p)),
        Key::Named(NamedKey::F7) => p.prompt = Some(Prompt::mkdir()),
        Key::Named(NamedKey::F8) => return Some(delete(p)),
        // Printable input builds up the command line (classic Far
        // behaviour); any edit cancels an in-flight `!` ask (the worker
        // thread still finishes in the background, but its result is now
        // dropped — see `FarPane::poll_ask`) and demotes a landed
        // suggestion back to plain, unhighlighted text ("keep typing to
        // edit").
        Key::Named(NamedKey::Space) => {
            p.cmdline.push(' ');
            p.complete = None;
            p.ask = None;
        }
        Key::Character(s) => {
            p.cmdline.push_str(s.as_str());
            p.complete = None;
            p.ask = None;
        }
        _ => {}
    }
    None
}

/// Handle a key while the drive-select overlay (Alt+F1/F2) is open: Up/Down
/// move the highlighted row, Enter applies it (`choose_drive`), Esc closes
/// the overlay without changing anything.
fn drive_select_key(p: &mut FarPane, key: &KeyEvent) -> Option<FarAction> {
    match &key.logical_key {
        Key::Named(NamedKey::Escape) => {
            p.drive_select = None;
            None
        }
        Key::Named(NamedKey::Enter) => p.choose_drive(),
        Key::Named(NamedKey::ArrowDown) => {
            if let Some(ds) = p.drive_select.as_mut() {
                if !ds.options.is_empty() {
                    ds.sel = (ds.sel + 1) % ds.options.len();
                }
            }
            None
        }
        Key::Named(NamedKey::ArrowUp) => {
            if let Some(ds) = p.drive_select.as_mut() {
                if !ds.options.is_empty() {
                    ds.sel = (ds.sel + ds.options.len() - 1) % ds.options.len();
                }
            }
            None
        }
        _ => None,
    }
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
    let side = p.active;
    let panel = p.active_panel_mut();
    let entry = panel.entries.get(panel.sel)?;
    let (is_parent, is_dir, name) = (entry.is_parent, entry.is_dir, entry.name.clone());
    if is_parent {
        return ascend(p);
    }
    if is_dir {
        panel.loc = panel.loc.child(&name);
        panel.sel = 0;
        panel.reload(); // no-op for a remote panel — its listing lands via begin_list below
        if p.panel(side).loc.is_remote() {
            return Some(p.begin_list(side));
        }
        return None;
    }
    match panel.loc.local_path() {
        Some(dir) => Some(FarAction::Open(dir.join(name))),
        None => Some(p.begin_download(&name)), // remote file: download, then open
    }
}

/// F3: show the selected file in the viewer pane (directories ignored); a
/// remote file downloads first (`begin_download`) — the same download path
/// F4 uses, since the download always lands as `FarAction::Open` regardless
/// of which key started it (see `remote::absorb_download`).
fn view_selected(p: &mut FarPane) -> Option<FarAction> {
    let (name, local) = selected_file(p)?;
    match local {
        Some(dir) => Some(FarAction::View(dir.join(&name))),
        None => Some(p.begin_download(&name)),
    }
}

/// F4: open the selected file with `$EDITOR` in a terminal pane (directories
/// ignored); a remote file downloads first (`begin_download`), same as F3.
fn edit_selected(p: &mut FarPane) -> Option<FarAction> {
    let (name, local) = selected_file(p)?;
    match local {
        Some(dir) => Some(FarAction::Edit(dir.join(&name))),
        None => Some(p.begin_download(&name)),
    }
}

/// Shared F3/F4 lookup: the selected entry's name and, when the active panel
/// is local, its parent directory — `None` for the parent (`..`) row or a
/// directory entry, which F3/F4 both ignore.
fn selected_file(p: &FarPane) -> Option<(String, Option<PathBuf>)> {
    let panel = p.panel(p.active);
    let entry = panel.entries.get(panel.sel)?;
    if entry.is_parent || entry.is_dir {
        return None;
    }
    Some((entry.name.clone(), panel.loc.local_path()))
}

/// Move the active panel up to its parent directory.
pub(crate) fn ascend(p: &mut FarPane) -> Option<FarAction> {
    let side = p.active;
    let panel = p.active_panel_mut();
    let parent = panel.loc.parent()?;
    panel.loc = parent;
    panel.sel = 0;
    panel.reload(); // no-op for a remote panel — its listing lands via begin_list below
    if p.panel(side).loc.is_remote() {
        return Some(p.begin_list(side));
    }
    None
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
