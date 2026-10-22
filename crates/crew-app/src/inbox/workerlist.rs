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
        if let Ok((email, h)) = g.profile() {
            self.history = h;
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
            Err(e) => fail(e),
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
