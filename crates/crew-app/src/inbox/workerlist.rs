//! The worker's list (`worker`): a page of the current search, the account
//! told with the first one that reaches Gmail, and where `j` at the end of
//! the list goes on from.
use super::super::api::Gmail;
use super::{fail, Got, State};

impl State {
    pub(super) fn list(&mut self, g: &mut Gmail, from: Option<String>) -> Vec<Got> {
        let more = from.is_some();
        let mut out = Vec::new();
        // Read before the page, so a mail landing mid-list moves it again.
        // Kept as the mailbox the pane's FIRST page saw — the minute's check
        // lists again only when it moved since. A further page (`j`) saw
        // none of the mail that landed on page one meanwhile, and a page
        // that failed was not seen at all: taking their history hid new mail
        // until something else moved the box, and turned a failed list into
        // "inbox zero" a minute later.
        let mut seen = None;
        if let Ok((email, h)) = g.profile() {
            seen = (!more).then_some(h);
            // Offline at the start, the account comes with the first list
            // that reaches Gmail: without it `me` is no one, so the list
            // never says "me" and reply-all writes to yourself as well.
            if !std::mem::replace(&mut self.told, true) {
                out.push(Got::Account(email));
            }
        }
        out.push(g.unread().map_or_else(fail, Got::Unread));
        out.push(match g.page(&self.query, from.as_deref()) {
            Ok((rows, next)) => {
                if let Some(h) = seen {
                    self.history = h;
                }
                let whole = next.is_none();
                self.went(more, next);
                Got::Page {
                    query: self.query.clone(),
                    rows,
                    next: self.next.clone(),
                    more,
                    whole,
                }
            }
            Err(e) => {
                if !more {
                    self.history.clear();
                }
                fail(e)
            }
        });
        out
    }

    /// Where `j` at the end of the list goes on from after a page: the page's
    /// own cursor, except that a first page listed again (the check after the
    /// mailbox moved) of a list already paged past it keeps the deeper one,
    /// as the pane keeps those rows. Gmail's end ends the list either way.
    pub(super) fn went(&mut self, more: bool, next: Option<String>) {
        match (more, next) {
            (false, Some(_)) if self.pages > 1 => {}
            (more, next) => {
                self.pages = if more { self.pages + 1 } else { 1 };
                self.next = next;
            }
        }
    }
}

#[cfg(test)]
#[path = "workerlist_tests.rs"]
mod tests;
