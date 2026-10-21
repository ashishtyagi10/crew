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
        match u.returns {
            true => {
                let at = u.at.min(self.rows.len());
                self.rows.insert(at, u.thread.clone());
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
        self.said(&u.thread, "put back");
    }
}
