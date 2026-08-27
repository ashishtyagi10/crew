//! Key reduction for the viewer, split into a pure seam the same way
//! `farpane::keys` is: `view_key` classifies a winit event into
//! `ViewInput`, and everything below that point is plain data the tests
//! drive directly.
//!
//! In-pane search (`/`, `n`, `N`) muddies the split a little: `view_key`
//! still has no notion of "a search is being typed" — it always classifies
//! `e`/`o`/`r`/`s`/`n`/`N`/`/` to their normal-mode `ViewInput` actions, so
//! `search_apply` is where those inputs get reinterpreted as the literal
//! characters they came from while `ViewPane::search` is mid-type (kept out
//! of this file to keep it to the scroll/action dispatch it's named for).
use std::path::PathBuf;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::viewpane::search::Search;
use crate::viewpane::search_apply;
use crate::viewpane::ViewPane;

/// A Page Up/Down jump: 10 rows.
const PAGE: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewInput {
    Close,
    Up,
    Down,
    PageUp,
    PageDown,
    Top,
    Bottom,
    Edit,
    OpenExternal,
    Reload,
    ToggleRaw,
    /// `v` — lay a diff out side by side, or unified again.
    ToggleSplit,
    /// `/` — start (or restart) a search in typing mode.
    Slash,
    /// `n` outside typing — jump to the next hit.
    NextHit,
    /// `N` outside typing — jump to the previous hit.
    PrevHit,
    /// Confirms a typed needle: stop typing and jump to the first hit.
    Enter,
    /// A printable character typed into a live needle.
    Char(char),
    /// `]` / `[`: the next / previous landmark ([`super::outline`]).
    NextMark,
    PrevMark,
    /// Deletes the last character of a live needle.
    Backspace,
    Ignore,
}

/// What the viewer asks the host app to do after a key press.
pub(crate) enum ViewAction {
    Close,
    /// `e` — open `$EDITOR` on this path in a terminal pane.
    Edit(PathBuf),
    /// `o` — hand this path to the OS default application.
    OpenExternal(PathBuf),
    Reload,
}

pub(crate) fn view_key(logical: &Key, pressed: bool, ctrl: bool) -> ViewInput {
    if !pressed {
        return ViewInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::Escape) => ViewInput::Close,
        Key::Named(NamedKey::ArrowUp) => ViewInput::Up,
        Key::Named(NamedKey::ArrowDown) => ViewInput::Down,
        Key::Named(NamedKey::PageUp) => ViewInput::PageUp,
        Key::Named(NamedKey::PageDown) => ViewInput::PageDown,
        Key::Named(NamedKey::Home) => ViewInput::Top,
        Key::Named(NamedKey::End) => ViewInput::Bottom,
        Key::Named(NamedKey::Enter) => ViewInput::Enter,
        Key::Named(NamedKey::Backspace) => ViewInput::Backspace,
        Key::Named(NamedKey::Space) => ViewInput::Char(' '),
        // `n`/`N` are checked by exact case (a search direction, like vim)
        // before the case-folded e/o/r/s block below.
        Key::Character(s) if !ctrl && s.as_str() == "n" => ViewInput::NextHit,
        Key::Character(s) if !ctrl && s.as_str() == "N" => ViewInput::PrevHit,
        Key::Character(s) if !ctrl && s.as_str() == "/" => ViewInput::Slash,
        // `]` / `[` step the document's structure: file to file and hunk to
        // hunk in a review. Not `n`/`N`, which belong to the search.
        Key::Character(s) if !ctrl && s.as_str() == "]" => ViewInput::NextMark,
        Key::Character(s) if !ctrl && s.as_str() == "[" => ViewInput::PrevMark,
        Key::Character(s) if !ctrl => match s.to_ascii_lowercase().as_str() {
            "e" => ViewInput::Edit,
            "o" => ViewInput::OpenExternal,
            "r" => ViewInput::Reload,
            "s" => ViewInput::ToggleRaw,
            "v" => ViewInput::ToggleSplit,
            _ => s
                .chars()
                .next()
                .map(ViewInput::Char)
                .unwrap_or(ViewInput::Ignore),
        },
        _ => ViewInput::Ignore,
    }
}

/// Scroll to the next (`down`) or previous landmark, putting it on the top
/// row. Nothing happens at either end, or in a document with no structure —
/// which is most of them, and is why this is silent rather than an error.
fn jump_mark(p: &mut ViewPane, cols: u16, rows: u16, down: bool) {
    let to = {
        let cache = p.lines_for(cols);
        super::outline::step(&cache.marks, p.scroll, down).map(|m| m.row)
    };
    if let Some(row) = to {
        p.scroll = row;
        p.clamp_scroll(cols, rows);
    }
}

/// Apply a classified press, returning an action when the host must act.
pub(crate) fn apply(
    p: &mut ViewPane,
    input: ViewInput,
    cols: u16,
    rows: u16,
) -> Option<ViewAction> {
    // A live search intercepts input before the normal scroll/action match:
    // Esc cancels it (rather than closing the pane — that's the second
    // Esc's job), and while it's still being typed every other input is a
    // needle edit, not its normal-mode meaning. See `search_apply`.
    if search_apply::intercept(p, input, cols, rows) {
        return None;
    }
    let scroll = |p: &mut ViewPane, d: i32| {
        p.scroll = p.scroll.saturating_add_signed(d as isize);
        p.clamp_scroll(cols, rows);
    };
    match input {
        ViewInput::Close => return Some(ViewAction::Close),
        ViewInput::Up => scroll(p, -1),
        ViewInput::Down => scroll(p, 1),
        ViewInput::PageUp => scroll(p, -PAGE),
        ViewInput::PageDown => scroll(p, PAGE),
        ViewInput::Top => p.scroll = 0,
        ViewInput::Bottom => scroll(p, i32::MAX / 2),
        ViewInput::Edit => return Some(ViewAction::Edit(p.path.clone())),
        ViewInput::OpenExternal => return Some(ViewAction::OpenExternal(p.path.clone())),
        ViewInput::Reload => return Some(ViewAction::Reload),
        ViewInput::ToggleRaw => {
            p.raw = !p.raw;
            p.cache.replace(None);
            // The render just changed shape (different gutter, different
            // wrap width) without the file changing — a live search's hits
            // are line indexes into the OLD rendering. Recompute rather
            // than clear: unlike `reload` (where the file itself changed
            // and the old hits are meaningless), the needle the user typed
            // is still exactly what they asked for.
            search_apply::recompute_hits(p, cols);
        }
        // The same shape change `s` makes, for the same reason: the search's
        // hits are line indexes into a rendering that no longer exists.
        ViewInput::ToggleSplit => {
            p.split = !p.split;
            p.cache.replace(None);
            search_apply::recompute_hits(p, cols);
        }
        ViewInput::Slash => {
            let mut search = Search::new(String::new(), Vec::new());
            search.typing = true;
            p.search = Some(search);
        }
        ViewInput::NextMark => jump_mark(p, cols, rows, true),
        ViewInput::PrevMark => jump_mark(p, cols, rows, false),
        ViewInput::NextHit => search_apply::jump(p, cols, rows, Search::next),
        ViewInput::PrevHit => search_apply::jump(p, cols, rows, Search::prev),
        ViewInput::Enter | ViewInput::Char(_) | ViewInput::Backspace | ViewInput::Ignore => {}
    }
    None
}

impl ViewPane {
    pub(crate) fn on_key(
        &mut self,
        event: &KeyEvent,
        cols: u16,
        rows: u16,
        ctrl: bool,
    ) -> Option<ViewAction> {
        let input = view_key(&event.logical_key, event.state.is_pressed(), ctrl);
        apply(self, input, cols, rows)
    }

    /// Mouse-wheel scrolling. A positive `lines` (documented "up/older" by
    /// `scroll::scroll_pane`, the caller) DECREASES the stored offset,
    /// toward the top — `self.scroll` counts rows down from the top, so
    /// scrolling up subtracts. Matched here rather than assumed: getting
    /// this sign backwards is invisible to every unit test and only shows
    /// up as scrolling the wrong way in the running app.
    pub(crate) fn scroll_wheel(&mut self, cols: u16, rows: u16, lines: i32) {
        self.scroll = self
            .scroll
            .saturating_add_signed(lines.saturating_neg() as isize);
        self.clamp_scroll(cols, rows);
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
