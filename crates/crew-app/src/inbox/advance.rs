//! Reading on after an archive or a delete (goal
//! 2026-10-21-inbox-delete-advances): the thread that takes the gone one's
//! place opens, as Gmail's auto-advance does, instead of the list. Next means
//! older, the row below in a newest-first list; past the last row, the one
//! above; with no row left, the list.
use super::worker::Ask;
use super::{InboxPane, Reading};

impl InboxPane {
    /// Open thread `i` of the list, as Enter does: asked of Gmail, marked
    /// read as Gmail marks it, selected, its pictures fresh. False when
    /// there is no such thread.
    pub(crate) fn open_at(&mut self, i: usize) -> bool {
        let Some(thread) = self.rows.get(i).cloned() else {
            return false;
        };
        self.sel = i;
        self.ask(Ask::Open(thread.id.clone()));
        self.opened(&thread.id);
        self.reading = Some(Reading::new(thread));
        self.pics.clear();
        true
    }

    /// After the open thread at `at` left the list: the next one, if any.
    pub(super) fn advance(&mut self, at: usize) {
        let next = match at < self.rows.len() {
            true => at,
            false => at.saturating_sub(1),
        };
        self.open_at(next);
    }
}

#[cfg(test)]
#[path = "advance_tests.rs"]
mod tests;
