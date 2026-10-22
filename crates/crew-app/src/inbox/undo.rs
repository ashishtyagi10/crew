//! `z`: the way back from the last filing (`triage`).
use super::wire::Thread;
use super::InboxPane;

/// The way back from the last change: the row as it was, where it was, and
/// the labels that restore it.
pub(crate) struct Undo {
    pub thread: Thread,
    pub at: usize,
    pub add: Vec<&'static str>,
    pub remove: Vec<&'static str>,
    /// The row left the list (an archive), so undo puts it back in.
    pub returns: bool,
    /// The thread went to Trash, so undo takes it out again.
    pub trashed: bool,
    /// It was open when it went: undo opens it again.
    pub reopen: bool,
}

impl InboxPane {
    /// `z`: the last change taken back. A thread that left the open reader
    /// (archived or deleted, the reader gone on to the next) opens again.
    pub(super) fn undo(&mut self) {
        let Some(u) = self.undo.take() else {
            self.note = "nothing to undo".into();
            return;
        };
        // Back already (a list that came again carried it): selected, not
        // added a second time.
        let back = self.rows.iter().position(|t| t.id == u.thread.id);
        match u.returns {
            true => {
                let at = back.unwrap_or_else(|| u.at.min(self.rows.len()));
                if back.is_none() {
                    self.rows.insert(at, u.thread.clone());
                }
                self.sel = at;
                if u.reopen {
                    self.open_at(at);
                }
            }
            false => {
                if let Some(row) = self.rows.iter_mut().find(|t| t.id == u.thread.id) {
                    *row = u.thread.clone();
                }
            }
        }
        match u.trashed {
            true => self.trash(&u.thread.id, true),
            false => self.relabel(&u.thread.id, u.add, u.remove),
        }
        // Taken back: nothing left for a second `z`.
        self.note = super::triage::of(&u.thread, "put back");
    }

    /// Gmail refused to file thread `id`: said, and when it is the last
    /// filing, the row put back as it was (the list Gmail sends again only
    /// reaches its first page) and its undo let go, as there is nothing left
    /// to take back.
    pub(super) fn refused(&mut self, id: &str, why: String) {
        if let Some(u) = self.undo.take_if(|u| u.thread.id == id) {
            match self.rows.iter().position(|t| t.id == id) {
                Some(i) => self.rows[i] = u.thread,
                None if u.returns => {
                    let at = u.at.min(self.rows.len());
                    // The selection stays on the thread it is on.
                    if at <= self.sel && !self.rows.is_empty() {
                        self.sel += 1;
                    }
                    self.rows.insert(at, u.thread);
                }
                None => {}
            }
        }
        self.note = format!("Gmail refused that, so it is as it was: {why}");
    }
}

#[cfg(test)]
#[path = "undo_tests.rs"]
mod tests;
