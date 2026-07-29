//! Key reduction for the viewer, split into a pure seam the same way
//! `mdkeys`/`farpane::keys` are: `view_key` classifies a winit event into
//! `ViewInput`, and everything below that point is plain data the tests
//! drive directly.
//!
//! In-pane search (`/`, `n`, `N`) muddies the split a little: `view_key`
//! still has no notion of "a search is being typed" — it always classifies
//! `e`/`o`/`r`/`s`/`n`/`N`/`/` to their normal-mode `ViewInput` actions — so
//! `apply` is the one place that reinterprets those inputs as the literal
//! characters they came from while `ViewPane::search` is mid-type. Each
//! dedicated variant maps to exactly one character, so the reverse mapping
//! in `apply_typing` is exact, not a guess.
use std::path::PathBuf;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::viewpane::search::Search;
use crate::viewpane::ViewPane;

/// A Page Up/Down jump, matching `mdkeys::PAGE`.
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
    /// Deletes the last character of a live needle.
    Backspace,
    Ignore,
}

/// What the viewer asks the host app to do after a key press.
pub(crate) enum ViewAction {
    Close,
    Status(String),
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
        Key::Character(s) if !ctrl => match s.to_ascii_lowercase().as_str() {
            "e" => ViewInput::Edit,
            "o" => ViewInput::OpenExternal,
            "r" => ViewInput::Reload,
            "s" => ViewInput::ToggleRaw,
            _ => s
                .chars()
                .next()
                .map(ViewInput::Char)
                .unwrap_or(ViewInput::Ignore),
        },
        _ => ViewInput::Ignore,
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
    // needle edit, not its normal-mode meaning.
    if p.search.is_some() {
        if matches!(input, ViewInput::Close) {
            p.search = None;
            return None;
        }
        if p.search.as_ref().is_some_and(|s| s.typing) {
            apply_typing(p, input, cols, rows);
            return None;
        }
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
        }
        ViewInput::Slash => {
            let mut search = Search::new(String::new(), Vec::new());
            search.typing = true;
            p.search = Some(search);
        }
        ViewInput::NextHit => jump(p, cols, rows, Search::next),
        ViewInput::PrevHit => jump(p, cols, rows, Search::prev),
        ViewInput::Enter | ViewInput::Char(_) | ViewInput::Backspace | ViewInput::Ignore => {}
    }
    None
}

/// `n`/`N` outside a live search do nothing — there is nothing to jump to.
fn jump(p: &mut ViewPane, cols: u16, rows: u16, step: impl FnOnce(&mut Search) -> Option<usize>) {
    if let Some(line) = p.search.as_mut().and_then(step) {
        p.scroll = line;
        p.clamp_scroll(cols, rows);
    }
}

/// While a search is being typed, every input is a needle edit — including
/// `e`/`o`/`r`/`s`/`n`/`N`/`/`, which `view_key` has already folded into
/// their normal-mode `ViewInput`s. Reverse-mapping them back to the one
/// character each represents is what lets you type a needle containing any
/// of those letters without firing Edit/Reload/ToggleRaw or jumping hits.
fn apply_typing(p: &mut ViewPane, input: ViewInput, cols: u16, rows: u16) {
    let Some(search) = p.search.as_mut() else {
        return;
    };
    match input {
        ViewInput::Char(c) => search.needle.push(c),
        ViewInput::Backspace => {
            search.needle.pop();
        }
        ViewInput::Edit => search.needle.push('e'),
        ViewInput::OpenExternal => search.needle.push('o'),
        ViewInput::Reload => search.needle.push('r'),
        ViewInput::ToggleRaw => search.needle.push('s'),
        ViewInput::NextHit => search.needle.push('n'),
        ViewInput::PrevHit => search.needle.push('N'),
        ViewInput::Slash => search.needle.push('/'),
        ViewInput::Enter => {
            search.typing = false;
            // Every scroll action clamps the stored offset, this jump
            // included — otherwise a hit past the last full page strands
            // `scroll` beyond content length and deadens later Up ticks.
            if let Some(line) = search.next() {
                p.scroll = line;
                p.clamp_scroll(cols, rows);
            }
            return;
        }
        // Close is handled by the caller before reaching here; scroll/no-op
        // inputs have no meaning while typing a needle.
        _ => return,
    }
    recompute_hits(p, cols);
}

/// Recompute `p.search`'s hits against the pane's current rendered text.
/// Called on every needle edit so `n`/`N` always walk what's on screen, not
/// a search that's gone stale since the last keystroke.
fn recompute_hits(p: &mut ViewPane, cols: u16) {
    let Some(needle) = p.search.as_ref().map(|s| s.needle.clone()) else {
        return;
    };
    // Scoped so the immutable borrow of `p` behind this `Ref` ends before
    // `p.search.as_mut()` below needs a mutable one.
    let lines: Vec<String> = {
        let cache = p.lines_for(cols);
        cache.lines.iter().map(line_text).collect()
    };
    let hits = crate::viewpane::search::find_matches(&lines, &needle);
    if let Some(search) = p.search.as_mut() {
        search.hits = hits;
    }
}

fn line_text(line: &crate::chatbody::CardLine) -> String {
    line.iter().map(|c| c.c).collect()
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

    /// Mouse-wheel scrolling. `MdPane::scroll_wheel` routes into
    /// `MdPane::scroll`, which computes `target - delta` — so a positive
    /// `lines` (documented "up/older" by `scroll::scroll_pane`) DECREASES
    /// the stored offset, toward the top. Matched here rather than assumed:
    /// getting this sign backwards is invisible to every unit test and only
    /// shows up as scrolling the wrong way in the running app.
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
