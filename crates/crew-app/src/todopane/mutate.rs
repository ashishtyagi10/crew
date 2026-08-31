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
                // A bumped due is a new deadline: let it toast again.
                it.notified = false;
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

    /// `e`: reload the item into the composer for an in-place edit. The due
    /// is appended as round-trippable text ([`super::duedate::edit_text`]) so
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
            .and_then(|d| super::duedate::edit_text(d, it.due_has_time))
        {
            s.push(' ');
            s.push_str(&txt);
        }
        self.cursor = s.chars().count();
        self.input = s;
        self.editing = Some(it.id);
        self.sel = None;
        self.tagmenu = None;
    }
}
