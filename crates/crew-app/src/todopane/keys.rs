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
    /// Unmodified paging keys — list traversal (the Shift chords stay the
    /// app-wide pane-scroll bindings and never reach the pane).
    PageUp,
    PageDown,
    /// `Home`/`Ctrl+A` and `End`/`Ctrl+E`: first/last item in the list,
    /// the draft's ends in the composer.
    Home,
    End,
    /// Composer cursor movement (list: inert).
    Left,
    Right,
    WordLeft,
    WordRight,
    Ignore,
}

/// An action the pane asks the app to take.
pub(crate) enum TodoAction {
    Close,
}

/// Classify a key press. Only presses act; Escape walks back one layer
/// (popup → edit/text → pane close, see [`apply`]). `alt` turns the arrows
/// into word jumps (macOS Alt mangles `Key::Character`, so word-jump must
/// key off the named arrow + flag); `ctrl` maps the terminal line idioms
/// `Ctrl+A`/`Ctrl+E` onto Home/End and swallows other Ctrl-letters.
pub(crate) fn todo_key(logical: &Key, pressed: bool, ctrl: bool, alt: bool) -> TodoInput {
    if !pressed {
        return TodoInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::ArrowLeft) if alt => TodoInput::WordLeft,
        Key::Named(NamedKey::ArrowRight) if alt => TodoInput::WordRight,
        Key::Named(NamedKey::ArrowLeft) => TodoInput::Left,
        Key::Named(NamedKey::ArrowRight) => TodoInput::Right,
        Key::Character(s) if ctrl => match s.chars().next() {
            Some('a') => TodoInput::Home,
            Some('e') => TodoInput::End,
            _ => TodoInput::Ignore,
        },
        Key::Named(NamedKey::Escape) => TodoInput::Close,
        Key::Named(NamedKey::Enter) => TodoInput::Enter,
        Key::Named(NamedKey::Backspace) => TodoInput::Backspace,
        Key::Named(NamedKey::Delete) => TodoInput::DeleteKey,
        Key::Named(NamedKey::Tab) => TodoInput::Tab,
        Key::Named(NamedKey::ArrowUp) => TodoInput::Up,
        Key::Named(NamedKey::ArrowDown) => TodoInput::Down,
        Key::Named(NamedKey::PageUp) => TodoInput::PageUp,
        Key::Named(NamedKey::PageDown) => TodoInput::PageDown,
        Key::Named(NamedKey::Home) => TodoInput::Home,
        Key::Named(NamedKey::End) => TodoInput::End,
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
                    p.cursor = p.input.chars().count();
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
            // In the done history Esc leaves the VIEW (the pane stays);
            // everywhere else it hands focus back to the composer.
            Close if p.done_view => p.set_done_view(false),
            Close | Tab => p.sel = None,
            Up => p.sel = (sel > 0).then(|| sel - 1),
            Down => p.sel = Some((sel + 1).min(p.visible_len().saturating_sub(1))),
            PageUp => p.sel = Some(p.page_target(sel, false, cols, rows)),
            PageDown => p.sel = Some(p.page_target(sel, true, cols, rows)),
            Home => p.sel = (p.visible_len() > 0).then_some(0),
            End => p.sel = (p.visible_len() > 0).then(|| p.visible_len() - 1),
            Left | Right | WordLeft | WordRight => {}
            Enter | Char(' ') => p.toggle_done_at(sel),
            Backspace | DeleteKey | Char('d') => p.delete_at(sel),
            // `e` would open an edit the history's composer can't submit,
            // and `h` interleaves done rows a done-only view already shows:
            // both inert in there (and NOT composer-jump printables).
            Char('e') | Char('h') if p.done_view => {}
            Char('e') => p.edit_at(sel),
            // `H`: flip into (or out of) the done-history log.
            Char('H') => p.set_done_view(!p.done_view),
            // `h` (list only — in the composer it just types): show/hide
            // done items. Hiding clamps a selection left stranded past the
            // shorter list.
            Char(']') => p.cycle_filter(true),
            Char('[') => p.cycle_filter(false),
            Char('+') => p.bump_due_at(sel, true),
            Char('-') => p.bump_due_at(sel, false),
            Char('h') => p.set_show_done(!p.show_done),
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
            } else if p.done_view {
                // One more layer to walk back: the history view itself.
                p.set_done_view(false);
            } else {
                return Some(TodoAction::Close);
            }
        }
        Enter => p.submit(),
        Backspace => p.backspace(),
        DeleteKey => p.delete_forward(),
        Char(c) => p.type_char(c),
        Left => p.cursor_move(-1),
        Right => p.cursor_move(1),
        WordLeft => p.cursor_word(false),
        WordRight => p.cursor_word(true),
        Home => p.cursor_ends(false),
        End => p.cursor_ends(true),
        // Up/Down first travel the wrapped draft; at its edges they move
        // into the list — Down from the last line enters at the top, Up
        // from the first at the bottom (the row nearest the composer).
        // Tab always hops to the list.
        Tab => {
            if p.visible_len() > 0 {
                p.sel = Some(0);
            }
        }
        Down => {
            if !p.cursor_vertical(false, cols) && p.visible_len() > 0 {
                p.sel = Some(0);
            }
        }
        Up => {
            let n = p.visible_len();
            if !p.cursor_vertical(true, cols) && n > 0 {
                p.sel = Some(n - 1);
            }
        }
        PageUp | PageDown | Ignore => {}
    }
    p.ensure_visible(cols, rows);
    None
}

impl TodoPane {
    pub(crate) fn on_key(
        &mut self,
        event: &KeyEvent,
        cols: u16,
        rows: u16,
        ctrl: bool,
        alt: bool,
    ) -> Option<TodoAction> {
        let input = todo_key(&event.logical_key, event.state.is_pressed(), ctrl, alt);
        apply(self, input, cols, rows)
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
