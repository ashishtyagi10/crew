//! Key reduction for the todo pane, split into the same pure seam as
//! `viewpane::keys`: [`todo_key`] classifies a winit event, [`apply`] is
//! plain data the tests drive directly.
//!
//! Two focus zones share the keyboard: the composer (default) and the list
//! (`sel` is `Some`). The tag popup, when open, wins arrows/Tab/Enter first;
//! its other keys fall through to composer editing so typing keeps
//! narrowing the query.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::{tagmenu, TodoPane};

/// What a key press means to a todo pane.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TodoInput {
    Close,
    Char(char),
    Enter,
    Backspace,
    /// The forward-Delete named key (deletes the selected item, like `d`).
    DeleteKey,
    Tab,
    Up,
    Down,
    Ignore,
}

/// An action the pane asks the app to take.
pub(crate) enum TodoAction {
    Close,
}

/// Classify a key press. Only presses act; Escape walks back one layer
/// (popup → edit/text → pane close, see [`apply`]).
pub(crate) fn todo_key(logical: &Key, pressed: bool) -> TodoInput {
    if !pressed {
        return TodoInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::Escape) => TodoInput::Close,
        Key::Named(NamedKey::Enter) => TodoInput::Enter,
        Key::Named(NamedKey::Backspace) => TodoInput::Backspace,
        Key::Named(NamedKey::Delete) => TodoInput::DeleteKey,
        Key::Named(NamedKey::Tab) => TodoInput::Tab,
        Key::Named(NamedKey::ArrowUp) => TodoInput::Up,
        Key::Named(NamedKey::ArrowDown) => TodoInput::Down,
        Key::Named(NamedKey::Space) => TodoInput::Char(' '),
        Key::Character(s) => s.chars().next().map_or(TodoInput::Ignore, TodoInput::Char),
        _ => TodoInput::Ignore,
    }
}

/// Apply a classified press. `cols`/`rows` are the pane's grid size, needed
/// to keep a moved selection scrolled into view (item heights depend on the
/// width — wrapped titles).
pub(crate) fn apply(
    p: &mut TodoPane,
    input: TodoInput,
    cols: u16,
    rows: u16,
) -> Option<TodoAction> {
    use TodoInput::*;
    // Popup first: navigation and acceptance; everything else falls through
    // so typing keeps editing the query.
    if let Some(m) = &mut p.tagmenu {
        match input {
            Up => {
                m.sel = m.sel.saturating_sub(1);
                return None;
            }
            Down => {
                m.sel = (m.sel + 1).min(m.matches.len().saturating_sub(1));
                return None;
            }
            Tab | Enter => {
                if let Some(tag) = m.matches.get(m.sel) {
                    p.input = tagmenu::accept(&p.input, tag);
                }
                p.tagmenu = None;
                return None;
            }
            Close => {
                p.tagmenu = None;
                return None;
            }
            _ => {}
        }
    }
    if let Some(sel) = p.sel {
        match input {
            Close | Tab => p.sel = None,
            Up => p.sel = (sel > 0).then(|| sel - 1),
            Down => p.sel = Some((sel + 1).min(p.visible_len().saturating_sub(1))),
            Enter | Char(' ') => p.toggle_done_at(sel),
            Backspace | DeleteKey | Char('d') => p.delete_at(sel),
            Char('e') => p.edit_at(sel),
            // Any other printable jumps back to the composer and types.
            Char(c) => {
                p.sel = None;
                p.type_char(c);
            }
            Ignore => {}
        }
        p.ensure_visible(cols, rows);
        return None;
    }
    // Composer.
    match input {
        Close => {
            if p.editing.is_some() || !p.input.is_empty() {
                p.cancel_edit();
            } else {
                return Some(TodoAction::Close);
            }
        }
        Enter => p.submit(),
        Backspace => p.backspace(),
        Char(c) => p.type_char(c),
        // Arrows/Tab move into the list: Down from the top, Up from the
        // bottom (the row nearest the composer).
        Down | Tab => {
            if p.visible_len() > 0 {
                p.sel = Some(0);
            }
        }
        Up => {
            let n = p.visible_len();
            if n > 0 {
                p.sel = Some(n - 1);
            }
        }
        DeleteKey | Ignore => {}
    }
    p.ensure_visible(cols, rows);
    None
}

impl TodoPane {
    pub(crate) fn on_key(&mut self, event: &KeyEvent, cols: u16, rows: u16) -> Option<TodoAction> {
        let input = todo_key(&event.logical_key, event.state.is_pressed());
        apply(self, input, cols, rows)
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
