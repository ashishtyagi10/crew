//! What happens to ONE todo: marking it done, bumping its due date, deleting
//! it, opening it for edit, and finding which one the cursor is on.
//!
//! Split from [`super::act`] for the line cap, along the line between acting
//! on the list and acting on an item.
use super::TodoPane;

impl TodoPane {
    pub(crate) fn id_at(&self, display_idx: usize) -> Option<u64> {
        self.order().get(display_idx).map(|&i| self.items[i].id)
    }

    /// `refresh` for a write made elsewhere (the Reminders sync, another
    /// pane, `t` in the inbox): the highlight stays on the todo it was on,
    /// wherever that todo now sorts. A key here still moves by row (a tick
    /// steps on to the next one), but rows that shift under the cursor
    /// unasked must not hand `d` another todo to delete. A todo that is gone
    /// leaves the highlight on the row it was, as before.
    pub(crate) fn follow(&mut self) {
        let id = self.sel.and_then(|s| self.id_at(s));
        self.refresh();
        let order = self.order();
        if let Some(at) = id.and_then(|id| order.iter().position(|&i| self.items[i].id == id)) {
            self.sel = Some(at);
        }
    }

    pub(crate) fn toggle_done_at(&mut self, display_idx: usize) {
        let Some(id) = self.id_at(display_idx) else {
            return;
        };
        let now_ms = crate::chattime::unix_now_ms();
        super::store::mutate(|items| {
            if let Some(it) = items.iter_mut().find(|it| it.id == id) {
                it.done = !it.done;
                // The completion stamp lives and dies with the tick: an
                // un-ticked item is open again, not "done at a stale time".
                it.done_ms = it.done.then_some(now_ms);
            }
        });
        self.refresh();
    }

    /// `+`/`-` on a selected row: postpone/advance its due one calendar
    /// day (wall clock kept). An undated item gets tomorrow at the default
    /// hour from `+`; `-` on undated stays undated — there is nothing to
    /// advance.
    pub(crate) fn bump_due_at(&mut self, display_idx: usize, forward: bool) {
        let Some(id) = self.id_at(display_idx) else {
            return;
        };
        let now = super::duedate::now_local();
        super::store::mutate(|items| {
            if let Some(it) = items.iter_mut().find(|it| it.id == id) {
                match it.due_ms {
                    Some(d) => {
                        it.due_ms =
                            Some(super::duedate::shift_days(d, if forward { 1 } else { -1 }))
                    }
                    None if forward => {
                        let tomorrow = now.date() + chrono::Duration::days(1);
                        it.due_ms = tomorrow
                            .and_hms_opt(super::duedate::DEFAULT_HOUR, 0, 0)
                            .and_then(super::duedate::to_epoch_ms);
                        it.due_has_time = false;
                    }
                    None => {}
                }
                // A bumped due is a new deadline, and toasts when it comes;
                // one still past is no news: `-` on an overdue row toasted it
                // again at once (the composer's rule, `act.rs`).
                let now_ms = crate::chattime::unix_now_ms();
                it.notified = it.due_ms.is_some_and(|d| d <= now_ms);
            }
        });
        self.refresh();
    }

    pub(crate) fn delete_at(&mut self, display_idx: usize) {
        let Some(id) = self.id_at(display_idx) else {
            return;
        };
        super::store::mutate(|items| items.retain(|it| it.id != id));
        self.refresh();
    }

    /// `e`: reload the item into the composer for an in-place edit. Every
    /// tag comes back under its own sigil and the due as round-trippable
    /// text ([`super::duedate::edit_text`]), so resubmitting unchanged keeps
    /// the item exactly as it was.
    pub(crate) fn edit_at(&mut self, display_idx: usize) {
        let Some(&idx) = self.order().get(display_idx) else {
            return;
        };
        let it = &self.items[idx];
        // The FULL chip list, not the row's: an item edited from under its
        // own band must not come back belonging to nobody. Chips first, so a
        // title's own `@types/node` is not read back as its project; the due
        // before the title unless the title names a date itself, so its last
        // word ("turn the heater on") is not read as the date's lead-in.
        let mut parts = super::rowchips::chips(it);
        let due = (it.due_ms).and_then(|d| super::duedate::edit_text(d, it.due_has_time));
        let dated = super::duedate::find(&it.title, super::duedate::now_local()).is_some();
        match due {
            Some(d) if !dated => parts.extend([d, it.title.clone()]),
            Some(d) => parts.extend([it.title.clone(), d]),
            None => parts.push(it.title.clone()),
        }
        let s = parts.join(" ");
        self.cursor = s.chars().count();
        self.input = s;
        self.editing = Some(it.id);
        self.sel = None;
        self.tagmenu = None;
    }
}

#[cfg(test)]
#[path = "follow_tests.rs"]
mod follow_tests;

#[cfg(test)]
#[path = "bump_tests.rs"]
mod bump_tests;
