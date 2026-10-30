//! Reading on after an archive or a delete (goal
//! 2026-10-21-inbox-delete-advances): the thread that takes the gone one's
//! place opens, as Gmail's auto-advance does, instead of the list. Next means
//! the row below (older, within the unread on top and the read under them,
//! `ontop`); past the last row, the one above; with no row left, the list.
use super::worker::Ask;
use super::{InboxPane, Reading};

/// Said when another thread is asked for while a reply is being written:
/// the box answers the thread it was opened on, so the mail under it stays.
pub(crate) const HELD: &str = "a reply is open \u{2014} send it, or esc to close it, first";
/// The same, for the todo card (`todocard`).
pub(crate) const TODO_HELD: &str =
    "a todo is open \u{2014} enter adds it, or esc lets it go, first";

impl InboxPane {
    /// Why another thread waits: the card open under this one.
    fn held(&self) -> &'static str {
        match self.todo {
            Some(_) => TODO_HELD,
            None => HELD,
        }
    }

    /// Open thread `i` of the list, as Enter does: asked of Gmail, marked
    /// read as Gmail marks it, selected, its pictures fresh. False when
    /// there is no such thread.
    pub(crate) fn open_at(&mut self, i: usize) -> bool {
        let Some(thread) = self.rows.get(i).cloned() else {
            return false;
        };
        let open = self.reading.as_ref().map(|r| r.thread.id.as_str());
        if self.carded() && open != Some(thread.id.as_str()) {
            self.note = self.held().into();
            return false;
        }
        self.select(i);
        self.ask(Ask::Open(thread.id.clone()));
        self.opened(&thread.id);
        self.reading = Some(Reading::new(thread));
        self.pics.clear();
        // `a`, `t` or `r` pressed on a thread that was then left with Esc is
        // forgotten: opened again later, it handed the mail to smith anyway.
        self.then = None;
        true
    }

    /// Move the selection to thread `i`, its wash gliding there (`motion`).
    pub(crate) fn select(&mut self, i: usize) {
        if i != self.sel {
            self.motion.glide = Some((self.sel, crate::anim::now_ms()));
            self.sel = i;
        }
    }

    /// `n` and `p` (or `J` and `K`) in the reader: the next thread (the row
    /// below) or the previous one, opened where the reader is, on
    /// any width (the user, 2026-10-21: "n go to next email … p open
    /// previous email in detail panel"). What to say: nothing when it opened.
    pub(crate) fn next_thread(&mut self, older: bool) -> &'static str {
        if self.carded() {
            return self.held();
        }
        // The open thread gone from the list: the one in its place is next.
        let open = self.reading.as_ref().map(|r| r.thread.id.as_str());
        let gone = self.rows.get(self.sel).map(|t| t.id.as_str()) != open;
        let to = match (older, self.sel.checked_sub(1)) {
            (true, _) if gone => self.sel,
            (true, _) => self.sel + 1,
            (false, Some(i)) => i,
            (false, None) => return "this is the top of the list",
        };
        if self.open_at(to) {
            return "";
        }
        if self.more && !self.busy {
            self.ask(Ask::More);
            return "loading older threads\u{2026} n again for the next";
        }
        "this is the end of the list"
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
