//! The `/todo` pane: one global todo list you type into.
//!
//! Enter on the composer creates an item; a natural-language date fragment
//! (`tomorrow`, `fri 5pm`, `aug 15` — see [`duedate`]) is tinted live and
//! becomes the due on save, and an `@project` token becomes a free-form tag
//! (autocompleted from tags already in use — see [`tagmenu`]). The list
//! sorts overdue → due → undated with done auto-hidden
//! ([`item::display_order`] — the store keeps done items as history);
//! Space/Enter toggles, `d`/Backspace deletes, `e` re-opens an item in the
//! composer. `@tag` alone filters the list to one project; `@` clears.
//!
//! State lives in the process-wide [`store`] (persisted `todos.toml`), so
//! every todo pane shows the same list and [`store::take_due`] can toast
//! due items app-side once a minute — event-driven, no per-frame work.
use crew_render::CellView;

mod act;
mod click;
mod composer;
pub(crate) mod duedate;
pub(crate) mod duetext;
mod edit;
pub(crate) mod fitline;
mod gutter;
pub(crate) mod item;
mod keys;
pub(crate) mod measure;
mod mutate;
pub(crate) mod render;
mod scrollpos;
pub(crate) mod store;
mod tagmenu;

pub(crate) use keys::TodoAction;
pub(crate) use render::TodoClick;

use item::TodoItem;
use tagmenu::TagMenu;

/// Throttle for the app-side due check: once a minute is plenty for a
/// minute-granular due instant, and it keeps the poll tick free of work.
const DUE_CHECK_EVERY_MS: u64 = 60_000;

/// The poll-tick clock behind the due-toast check (a `CrewApp` field).
#[derive(Default)]
pub(crate) struct DueTicker {
    next_ms: u64,
}

impl DueTicker {
    /// At most one check per [`DUE_CHECK_EVERY_MS`].
    pub(crate) fn due(&mut self, now: u64) -> bool {
        if now < self.next_ms {
            return false;
        }
        self.next_ms = now + DUE_CHECK_EVERY_MS;
        true
    }
}

/// A todo pane: the composer text and view state; the items themselves are
/// a snapshot of the shared [`store`], resynced by [`TodoPane::poll`].
pub struct TodoPane {
    /// Composer text.
    pub(crate) input: String,
    /// Composer cursor, a char index into `input` (0 ..= char count).
    /// Every text mutation goes through [`Self::insert_at_cursor`] /
    /// [`Self::backspace`] / [`Self::delete_forward`] so it can't drift.
    pub(crate) cursor: usize,
    /// Store snapshot this pane renders from (see [`Self::poll`]).
    pub(crate) items: Vec<TodoItem>,
    seen_rev: u64,
    /// Selected row as an index into the display order; `None` = composer.
    pub(crate) sel: Option<usize>,
    /// Item id being edited in the composer (Enter replaces, Esc cancels).
    pub(crate) editing: Option<u64>,
    /// The open `@project` completion popup.
    pub(crate) tagmenu: Option<TagMenu>,
    /// Active `@project` list filter.
    pub(crate) filter: Option<String>,
    /// Show done items (sunk, dimmed) — `h` in the list toggles. Off by
    /// default: done auto-hides (0.15.1), this is the way back.
    pub(crate) show_done: bool,
    /// The done-history view (`/todo done`, `H` in the list): the list
    /// becomes a done-only log — newest completion first under day headers —
    /// and the composer only filters. Esc leaves it.
    pub(crate) done_view: bool,
    /// First visible display-order index of the list.
    pub(crate) scroll: usize,
}

impl TodoPane {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            items: store::snapshot(),
            seen_rev: store::revision(),
            sel: None,
            editing: None,
            tagmenu: None,
            filter: None,
            show_done: false,
            done_view: false,
            scroll: 0,
        }
    }

    /// Resync from the shared store when its revision moved (another pane,
    /// or the due ticker, wrote). Returns whether a redraw is due.
    pub(crate) fn poll(&mut self) -> bool {
        if store::revision() == self.seen_rev {
            return false;
        }
        self.refresh();
        true
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::cells(self, cols, rows)
    }

    /// Paste inserts at the cursor (newlines become spaces — one line).
    pub(crate) fn paste(&mut self, text: &str) {
        let flat: String = text
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        self.insert_at_cursor(&flat);
    }

    // --- display order ---------------------------------------------------

    // --- composer editing -------------------------------------------------

    // --- item ops (shared by keys and clicks) -----------------------------
}

/// A pane over explicit items, bypassing the shared store — for tests that
/// only exercise pure view logic (layout, clicks, rendering).
#[cfg(test)]
pub(crate) fn test_pane(items: Vec<TodoItem>) -> TodoPane {
    TodoPane {
        input: String::new(),
        cursor: 0,
        items,
        seen_rev: 0,
        sel: None,
        editing: None,
        tagmenu: None,
        filter: None,
        show_done: false,
        done_view: false,
        scroll: 0,
    }
}

/// Split the first `@token` out of `text`: (title without it, the tag).
/// Later `@tokens` stay title text — one project per item.
pub(crate) fn extract_tag(text: &str) -> (String, Option<String>) {
    let mut title: Vec<&str> = Vec::new();
    let mut tag: Option<String> = None;
    for w in text.split_whitespace() {
        match (tag.is_none(), w.strip_prefix('@')) {
            (true, Some(t)) if !t.is_empty() => tag = Some(t.to_string()),
            _ => title.push(w),
        }
    }
    (title.join(" "), tag)
}

impl crate::app::CrewApp {}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

impl crate::app::CrewApp {}
