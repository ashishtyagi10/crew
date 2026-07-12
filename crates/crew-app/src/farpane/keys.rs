//! Key reduction for the Far pane: panel switching, cursor movement, descending
//! into directories / opening files, the classic function-key actions
//! (copy/move/delete/make-folder/view/edit/help), Tab completion + Up/Down
//! history + Right/End ghost-text acceptance on the command line, and closing
//! the pane.
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
        // activates the selected entry (descend / open). A landed
        // suggestion's text never starts with `!` (it's the bare command),
        // so this falls straight through to `run_cmdline` on accept.
        Key::Named(NamedKey::Enter) => {
            if typing {
                if let Some(desc) = super::ask::bang_ask(&p.cmdline) {
                    let desc = desc.to_string();
                    return Some(super::run::submit_ask(p, &desc));
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
                ascend(p);
            }
        }
        // F3 View / F4 Edit both open the selected file with the OS default app.
        Key::Named(NamedKey::F3) | Key::Named(NamedKey::F4) => return open_selected(p),
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

/// Esc on the command line: cancel an active Tab-cycle (restoring the
/// pre-cycle text) if one is running; else clear a typed command; else ask
/// the app to close the pane.
pub(crate) fn escape_cmdline(p: &mut FarPane) -> Option<FarAction> {
    if let Some(state) = p.complete.take() {
        p.cmdline = state.prefix;
        return None;
    }
    // A landed suggestion restores the original `!` text verbatim and
    // discards the suggestion. A still-thinking ask just cancels (its
    // worker thread finishes in the background but the result is dropped)
    // and falls through to the normal clear/close behaviour below.
    if let Some(super::ask::AskState::Suggested { original }) = p.ask.take() {
        p.cmdline = original;
        return None;
    }
    if !p.cmdline.is_empty() {
        p.cmdline.clear();
        return None;
    }
    Some(FarAction::Close)
}

/// Tab while the command line has text: cycle an existing candidate list, or
/// build a fresh one from the caret token. A single candidate applies
/// immediately (trailing space unless it's a directory, so deeper completion
/// chains); more than one starts a cycle on the first candidate, and another
/// Tab advances it (wrapping).
pub(crate) fn tab_complete(p: &mut FarPane) {
    if let Some(state) = &mut p.complete {
        state.i = (state.i + 1) % state.candidates.len();
        let candidate = state.candidates[state.i].clone();
        p.cmdline = super::complete::apply(&state.prefix, &candidate);
        return;
    }
    let (kind, _token) = super::complete::caret_token(&p.cmdline);
    let binaries = command_binaries(p, kind);
    let candidates = super::complete::candidates(&p.cmdline, &p.active_cwd(), &binaries);
    if candidates.is_empty() {
        return;
    }
    if candidates.len() == 1 {
        p.cmdline = super::complete::apply(&p.cmdline, &candidates[0]);
        if !p.cmdline.ends_with('/') {
            p.cmdline.push(' ');
        }
        return;
    }
    let prefix = p.cmdline.clone();
    p.cmdline = super::complete::apply(&prefix, &candidates[0]);
    p.complete = Some(super::complete::CycleState {
        candidates,
        i: 0,
        prefix,
    });
}

/// Command-kind completion needs the cached `$PATH` binaries; kick off the
/// background scan on first use (returns builtins-only until it lands, never
/// blocking this thread). Path-kind completion needs no binaries at all.
/// `p.bins` is the session-wide cache (see `FarPane::bins`/`shared_bins`), so
/// if another pane's scan already landed this returns instantly without
/// spawning anything here.
fn command_binaries(p: &mut FarPane, kind: super::complete::TokenKind) -> Vec<String> {
    if kind != super::complete::TokenKind::Command {
        return Vec::new();
    }
    if let Some(bins) = p.bins.get() {
        return bins.clone();
    }
    if !p.bins_scan_started {
        p.bins_scan_started = true;
        let slot = p.bins.clone();
        std::thread::spawn(move || {
            let path_var = std::env::var("PATH").unwrap_or_default();
            let bins = super::complete::scan_path_binaries(&path_var);
            let _ = slot.set(bins);
        });
    }
    Vec::new()
}

/// Up while typing: recall the previous (older) history entry into the
/// command line, stashing the currently-typed text so Down can restore it.
pub(crate) fn history_prev(p: &mut FarPane) {
    if let Some(s) = p.history.prev(&p.cmdline) {
        p.cmdline = s.to_string();
    }
    p.complete = None;
}

/// Down while typing: recall the next (newer) history entry, or restore the
/// text that was being typed once past the newest entry.
pub(crate) fn history_next(p: &mut FarPane) {
    if let Some(s) = p.history.next(&p.cmdline) {
        p.cmdline = s.to_string();
    }
    p.complete = None;
}

/// Right/End while typing: accept the visible ghost-text history suggestion
/// into the command line, if one is showing. `render.rs` suppresses the
/// ghost display while a Tab-cycle is active (the candidate list already
/// occupies the line), so during a cycle this must only end the cycle — a
/// ghost lookup here would insert a suggestion that was never on screen.
pub(crate) fn accept_ghost(p: &mut FarPane) {
    if p.complete.take().is_some() {
        return;
    }
    if let Some(g) = p.history.ghost(&p.cmdline) {
        p.cmdline = g.to_string();
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
