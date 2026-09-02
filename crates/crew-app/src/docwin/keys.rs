//! What a key means to a document being edited.
//!
//! Split from [`super::event`] so the classifier is one table a reader can check against the
//! manual, and the handling is somewhere else. Nothing here touches the document: it turns a
//! key and a modifier state into an intention, and every intention is a variant.
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::viewpane::caret::Step;

/// What a key means to a document being edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Edit {
    Move(Step),
    Type(String),
    /// The same movement, dragging a selection behind it.
    Select(Step),
    SelectAll,
    Backspace,
    Delete,
    Newline,
    Save,
    Undo,
    Redo,
    /// Wrap (or unwrap) the selection in this marker — `**` or `*`.
    Wrap(&'static str),
    /// Tab: the next table cell, or a tab's worth of spaces when there is none.
    Tab,
    /// Cmd+K: edit the URL of the link the caret is in, or make one out of the selection.
    Link,
    Copy,
    Cut,
    Paste,
}

/// Classify a key press for an editing window. `None` leaves it to the
/// viewer's own keys (Esc, the search, the scroll), which still apply.
pub(crate) fn edit_key(
    k: &winit::event::KeyEvent,
    mods: ModifiersState,
    rows: u16,
) -> Option<Edit> {
    // A page keeps one row of the last page in view, like every pager.
    let page = usize::from(rows).saturating_sub(1).max(1);
    edit_for(&k.logical_key, k.state.is_pressed(), mods, page)
}

/// [`edit_key`] over a key the tests can build — winit's `KeyEvent` is
/// `#[non_exhaustive]` and carries a platform field with no `Default`.
pub(crate) fn edit_for(
    key: &Key,
    pressed: bool,
    mods: ModifiersState,
    page: usize,
) -> Option<Edit> {
    if !pressed {
        return None;
    }
    let cmd = mods.super_key() || mods.control_key();
    let moved = |s: Step| match mods.shift_key() {
        true => Some(Edit::Select(s)),
        false => Some(Edit::Move(s)),
    };
    match key {
        // Alt+arrow hops a word, as it does in every composer crew has.
        Key::Named(NamedKey::ArrowLeft) if mods.alt_key() => moved(Step::WordLeft),
        Key::Named(NamedKey::ArrowRight) if mods.alt_key() => moved(Step::WordRight),
        // Cmd+arrow is the Mac's spelling of Home/End and the document's ends.
        Key::Named(NamedKey::ArrowLeft) if cmd => moved(Step::Home),
        Key::Named(NamedKey::ArrowRight) if cmd => moved(Step::End),
        Key::Named(NamedKey::ArrowUp) if cmd => moved(Step::Top),
        Key::Named(NamedKey::ArrowDown) if cmd => moved(Step::Bottom),
        Key::Named(NamedKey::ArrowLeft) => moved(Step::Left),
        Key::Named(NamedKey::ArrowRight) => moved(Step::Right),
        Key::Named(NamedKey::ArrowUp) => moved(Step::Up),
        Key::Named(NamedKey::ArrowDown) => moved(Step::Down),
        Key::Named(NamedKey::Home) => moved(Step::Home),
        Key::Named(NamedKey::End) => moved(Step::End),
        // The viewer scrolls on these; an editor moves the caret, or the
        // next arrow key would snap the view straight back to it.
        Key::Named(NamedKey::PageUp) => moved(Step::Page {
            down: false,
            rows: page,
        }),
        Key::Named(NamedKey::PageDown) => moved(Step::Page {
            down: true,
            rows: page,
        }),
        Key::Named(NamedKey::Backspace) if !cmd => Some(Edit::Backspace),
        Key::Named(NamedKey::Delete) if !cmd => Some(Edit::Delete),
        Key::Named(NamedKey::Enter) if !cmd => Some(Edit::Newline),
        Key::Named(NamedKey::Space) if !cmd => Some(Edit::Type(" ".into())),
        Key::Named(NamedKey::Tab) if !cmd => Some(Edit::Tab),
        Key::Character(s) if cmd => match s.as_str() {
            "s" => Some(Edit::Save),
            "a" => Some(Edit::SelectAll),
            "c" => Some(Edit::Copy),
            "x" => Some(Edit::Cut),
            "v" => Some(Edit::Paste),
            // The markers never appear on screen, so this is the way one gets
            // into the file at all.
            "b" => Some(Edit::Wrap("**")),
            "i" => Some(Edit::Wrap("*")),
            "k" => Some(Edit::Link),
            "z" => Some(Edit::Undo),
            // Cmd+Shift+Z arrives as the shifted character, exactly like the
            // `{`/`}` and `T` chords the grid uses.
            "Z" => Some(Edit::Redo),
            _ => None,
        },
        // A letter is a letter. Without the caret this is `r` for reload and
        // `o` for open-externally; with one, the window is an editor and
        // typing `o` has to type an `o`.
        Key::Character(s) => Some(Edit::Type(s.to_string())),
        _ => None,
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
