//! What the todo pane does to the LIST: submitting a new item, reloading, and
//! cycling which items it shows.
//!
//! Its sibling [`super::mutate`] holds what happens to ONE item.
//!
//! Split out of [`super`] for the line cap.
use super::item::TodoItem;

use super::{extract_tag, TodoPane};

impl TodoPane {
    /// Flip the done-history view (`H` on the list, Esc inside, `/todo
    /// done`). Entering selects the newest completion; leaving returns to
    /// the composer.
    pub(crate) fn set_done_view(&mut self, on: bool) {
        if self.done_view == on {
            return;
        }
        self.done_view = on;
        self.scroll = 0;
        self.sel = if on {
            (self.visible_len() > 0).then_some(0)
        } else {
            None
        };
    }

    /// Show or hide the done items inline (`h` on the list, the header's
    /// `[show N done]` button, `/todo show` and `/todo hide`). Hiding clamps
    /// a selection left stranded past the shorter list.
    pub(crate) fn set_show_done(&mut self, on: bool) {
        if self.show_done == on {
            return;
        }
        self.show_done = on;
        let n = self.visible_len();
        self.sel = self.sel.and_then(|s| (n > 0).then(|| s.min(n - 1)));
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
        if self.done_view {
            // The history's composer is a filter box: `@tag`/`@` above
            // acted, anything else must not mint an item out of the log.
            return;
        }
        // Parse the RAW input so what was highlighted is exactly what is
        // stripped (find/strip share char indices with the composer tint).
        let hit = super::duedate::find(&self.input, super::duedate::now_local());
        let rest = match &hit {
            Some(h) => super::duedate::strip(&self.input, h.start, h.end),
            None => trimmed.split_whitespace().collect::<Vec<_>>().join(" "),
        };
        let (title, project) = extract_tag(&rest);
        if title.is_empty() {
            return; // a due date or tag alone is not an item
        }
        let due_ms = hit
            .as_ref()
            .and_then(|h| super::duedate::to_epoch_ms(h.due));
        let has_time = hit.as_ref().is_some_and(|h| h.has_time);
        let now_ms = crate::chattime::unix_now_ms();
        match self.editing {
            Some(id) => super::store::mutate(|items| {
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
            None => super::store::mutate(|items| {
                let id = now_ms.max(items.iter().map(|i| i.id).max().map_or(0, |m| m + 1));
                items.push(TodoItem {
                    id,
                    title,
                    done: false,
                    done_ms: None,
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

    /// `[`/`]` on the list: cycle the `@project` filter through the known
    /// tags (usage order, the tag popup's own) with "no filter" as one stop
    /// on the ring. The selection re-enters at the top of the newly
    /// filtered list (or leaves the list when it filtered to empty).
    pub(crate) fn cycle_filter(&mut self, forward: bool) {
        let tags = super::tagmenu::known_tags(&self.items);
        if tags.is_empty() {
            return;
        }
        // Ring: None, tags[0], tags[1], … — position of the current stop.
        let here = match &self.filter {
            None => 0,
            Some(f) => tags
                .iter()
                .position(|t| t.eq_ignore_ascii_case(f))
                .map(|i| i + 1)
                .unwrap_or(0),
        };
        let n = tags.len() + 1;
        let next = if forward {
            (here + 1) % n
        } else {
            (here + n - 1) % n
        };
        self.filter = (next > 0).then(|| tags[next - 1].clone());
        self.scroll = 0;
        self.sel = (self.visible_len() > 0).then_some(0);
    }

    /// Re-snapshot the store and keep selection/scroll in range.
    pub(crate) fn refresh(&mut self) {
        self.items = super::store::snapshot();
        self.seen_rev = super::store::revision();
        let n = self.visible_len();
        self.sel = match (self.sel, n) {
            (_, 0) | (None, _) => None,
            (Some(s), n) => Some(s.min(n - 1)),
        };
        self.scroll = self.scroll.min(self.visible_len().saturating_sub(1));
    }
}
