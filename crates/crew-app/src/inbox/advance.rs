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
        self.select(i);
        self.ask(Ask::Open(thread.id.clone()));
        self.opened(&thread.id);
        self.reading = Some(Reading::new(thread));
        self.pics.clear();
        true
    }

    /// Move the selection to thread `i`, its wash gliding there (`motion`).
    pub(crate) fn select(&mut self, i: usize) {
        if i != self.sel {
            self.motion.glide = Some((self.sel, crate::anim::now_ms()));
            self.sel = i;
        }
    }

    /// `J` and `K` in the reader: the next thread (older, the row below) or
    /// the previous one, opened where the reader is, on any width. What to
    /// say: nothing when it opened.
    pub(crate) fn next_thread(&mut self, older: bool) -> &'static str {
        let to = match (older, self.sel.checked_sub(1)) {
            (true, _) => self.sel + 1,
            (false, Some(i)) => i,
            (false, None) => return "this is the newest thread",
        };
        if self.open_at(to) {
            return "";
        }
        if self.more && !self.busy {
            self.ask(Ask::More);
            return "loading older threads\u{2026} J again for the next";
        }
        "this is the oldest thread"
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
