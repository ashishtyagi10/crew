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

pub(crate) mod duedate;
pub(crate) mod item;
mod keys;
mod render;
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
    /// First visible display-order index of the list.
    pub(crate) scroll: usize,
}

impl TodoPane {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            items: store::snapshot(),
            seen_rev: store::revision(),
            sel: None,
            editing: None,
            tagmenu: None,
            filter: None,
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

    /// Mouse wheel: positive `lines` means up/older (see `scroll::scroll_pane`).
    /// Item-granular — one wheel line steps one item.
    pub(crate) fn scroll_wheel(&mut self, lines: i32, cols: u16, rows: u16) {
        self.scroll = self
            .scroll
            .saturating_add_signed(lines.saturating_neg() as isize);
        self.clamp_scroll(cols, rows);
    }

    /// Paste appends to the composer (newlines become spaces — one line).
    pub(crate) fn paste(&mut self, text: &str) {
        for c in text.chars() {
            self.input
                .push(if c == '\n' || c == '\r' { ' ' } else { c });
        }
        self.sync_menu();
    }

    // --- display order ---------------------------------------------------

    /// Item indices in display order under the active filter.
    pub(crate) fn order(&self) -> Vec<usize> {
        item::display_order(&self.items, self.filter.as_deref())
    }

    pub(crate) fn visible_len(&self) -> usize {
        self.order().len()
    }

    /// Keep the selection inside the list window after it moves. Items are
    /// variable-height (wrapped titles — [`render::item_h`]), so visibility
    /// is a row sum, not an item count.
    pub(crate) fn ensure_visible(&mut self, cols: u16, rows: u16) {
        let h = render::list_height(self, rows) as usize;
        if h == 0 {
            return;
        }
        let order = self.order();
        let now_ms = crate::chattime::unix_now_ms();
        if let Some(s) = self.sel.filter(|&s| s < order.len()) {
            if s < self.scroll {
                self.scroll = s;
            } else {
                // Push the window down until items scroll..=s fit it (a
                // single over-tall item stops at its first row).
                let span = |from: usize| -> usize {
                    order[from..=s]
                        .iter()
                        .map(|&i| render::item_h(&self.items[i], cols, now_ms) as usize)
                        .sum()
                };
                while self.scroll < s && span(self.scroll) > h {
                    self.scroll += 1;
                }
            }
        }
        self.clamp_scroll(cols, rows);
    }

    /// Cap `scroll` at the smallest value that still fills the window with
    /// the list's tail.
    fn clamp_scroll(&mut self, cols: u16, rows: u16) {
        let h = render::list_height(self, rows) as usize;
        let order = self.order();
        let now_ms = crate::chattime::unix_now_ms();
        let mut used = 0;
        let mut s = order.len();
        while s > 0 {
            let ih = render::item_h(&self.items[order[s - 1]], cols, now_ms) as usize;
            if used + ih > h {
                break;
            }
            used += ih;
            s -= 1;
        }
        self.scroll = self.scroll.min(s.min(order.len().saturating_sub(1)));
    }

    // --- composer editing -------------------------------------------------

    pub(crate) fn type_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        self.input.push(c);
        self.sync_menu();
    }

    pub(crate) fn backspace(&mut self) {
        self.input.pop();
        self.sync_menu();
    }

    fn sync_menu(&mut self) {
        let items = &self.items;
        tagmenu::after_edit(&mut self.tagmenu, &self.input, || {
            tagmenu::known_tags(items)
        });
    }

    fn reset_input(&mut self) {
        self.input.clear();
        self.editing = None;
        self.tagmenu = None;
    }

    /// Esc in the composer: drop the draft (and any edit in progress).
    pub(crate) fn cancel_edit(&mut self) {
        self.reset_input();
    }

    /// Enter in the composer: `@` clears the filter, a lone `@tag` sets it,
    /// anything else becomes (or, while editing, replaces) an item — the
    /// highlighted date fragment stripped into the due, the first `@token`
    /// into the project tag.
    pub(crate) fn submit(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.editing.is_none() {
            if trimmed == "@" {
                self.filter = None;
                self.reset_input();
                return;
            }
            if let Some(tag) = trimmed
                .strip_prefix('@')
                .filter(|t| !t.is_empty() && !t.contains(char::is_whitespace))
            {
                self.filter = Some(tag.to_string());
                self.reset_input();
                self.scroll = 0;
                self.sel = None;
                return;
            }
        }
        // Parse the RAW input so what was highlighted is exactly what is
        // stripped (find/strip share char indices with the composer tint).
        let hit = duedate::find(&self.input, duedate::now_local());
        let rest = match &hit {
            Some(h) => duedate::strip(&self.input, h.start, h.end),
            None => trimmed.split_whitespace().collect::<Vec<_>>().join(" "),
        };
        let (title, project) = extract_tag(&rest);
        if title.is_empty() {
            return; // a due date or tag alone is not an item
        }
        let due_ms = hit.as_ref().and_then(|h| duedate::to_epoch_ms(h.due));
        let has_time = hit.as_ref().is_some_and(|h| h.has_time);
        let now_ms = crate::chattime::unix_now_ms();
        match self.editing {
            Some(id) => store::mutate(|items| {
                if let Some(it) = items.iter_mut().find(|it| it.id == id) {
                    it.title = title;
                    it.project = project;
                    if it.due_ms != due_ms {
                        // A moved due gets its toast back.
                        it.notified = due_ms.is_some_and(|d| d <= now_ms);
                    }
                    it.due_ms = due_ms;
                    it.due_has_time = has_time;
                }
            }),
            None => store::mutate(|items| {
                let id = now_ms.max(items.iter().map(|i| i.id).max().map_or(0, |m| m + 1));
                items.push(TodoItem {
                    id,
                    title,
                    done: false,
                    project,
                    due_ms,
                    due_has_time: has_time,
                    created_ms: now_ms,
                    // Born already-past (e.g. `today 5am` typed at noon):
                    // it renders overdue — a toast on top would be noise.
                    notified: due_ms.is_some_and(|d| d <= now_ms),
                });
            }),
        }
        self.reset_input();
        self.refresh();
    }

    // --- item ops (shared by keys and clicks) -----------------------------

    fn id_at(&self, display_idx: usize) -> Option<u64> {
        self.order().get(display_idx).map(|&i| self.items[i].id)
    }

    pub(crate) fn toggle_done_at(&mut self, display_idx: usize) {
        let Some(id) = self.id_at(display_idx) else {
            return;
        };
        store::mutate(|items| {
            if let Some(it) = items.iter_mut().find(|it| it.id == id) {
                it.done = !it.done;
            }
        });
        self.refresh();
    }

    pub(crate) fn delete_at(&mut self, display_idx: usize) {
        let Some(id) = self.id_at(display_idx) else {
            return;
        };
        store::mutate(|items| items.retain(|it| it.id != id));
        self.refresh();
    }

    /// `e`: reload the item into the composer for an in-place edit. The due
    /// is appended as round-trippable text ([`duedate::edit_text`]) so
    /// resubmitting unchanged keeps it.
    pub(crate) fn edit_at(&mut self, display_idx: usize) {
        let Some(&idx) = self.order().get(display_idx) else {
            return;
        };
        let it = &self.items[idx];
        let mut s = it.title.clone();
        if let Some(p) = &it.project {
            s.push_str(&format!(" @{p}"));
        }
        if let Some(txt) = it
            .due_ms
            .and_then(|d| duedate::edit_text(d, it.due_has_time))
        {
            s.push(' ');
            s.push_str(&txt);
        }
        self.input = s;
        self.editing = Some(it.id);
        self.sel = None;
        self.tagmenu = None;
    }

    /// Re-snapshot the store and keep selection/scroll in range.
    fn refresh(&mut self) {
        self.items = store::snapshot();
        self.seen_rev = store::revision();
        let n = self.visible_len();
        self.sel = match (self.sel, n) {
            (_, 0) | (None, _) => None,
            (Some(s), n) => Some(s.min(n - 1)),
        };
        self.scroll = self.scroll.min(self.visible_len().saturating_sub(1));
    }
}

/// A pane over explicit items, bypassing the shared store — for tests that
/// only exercise pure view logic (layout, clicks, rendering).
#[cfg(test)]
pub(crate) fn test_pane(items: Vec<TodoItem>) -> TodoPane {
    TodoPane {
        input: String::new(),
        items,
        seen_rev: 0,
        sel: None,
        editing: None,
        tagmenu: None,
        filter: None,
        scroll: 0,
    }
}

/// Split the first `@token` out of `text`: (title without it, the tag).
/// Later `@tokens` stay title text — one project per item.
fn extract_tag(text: &str) -> (String, Option<String>) {
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

impl crate::app::CrewApp {
    /// A left click inside a todo pane acts where it lands: the checkbox
    /// toggles, the `✗` deletes, a row selects, the composer refocuses —
    /// and the pane takes focus. Empty regions return `false` and fall
    /// through to the normal focus/drag path.
    pub(crate) fn todo_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        if !matches!(self.panes[i].content, crate::pane::PaneContent::Todo(_)) {
            return false;
        }
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let grid = self.panes[i].grid;
        let crate::pane::PaneContent::Todo(t) = &mut self.panes[i].content else {
            return false;
        };
        let Some(click) = render::click_at(t, row as u16, col as u16, grid.cols, grid.rows) else {
            return false;
        };
        match click {
            TodoClick::Toggle(d) => t.toggle_done_at(d),
            TodoClick::Delete(d) => t.delete_at(d),
            TodoClick::Select(d) => t.sel = Some(d),
            TodoClick::Composer => t.sel = None,
        }
        self.focused = i;
        self.input.focused = false;
        true
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
