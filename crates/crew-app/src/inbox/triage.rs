//! Filing mail from the pane: `e` archives, `d` deletes, `m` marks read or
//! unread, `s` stars or unstars, `u` puts the last of those back. Each shows
//! at once and goes to Gmail behind it; a refusal is said and the list comes
//! back as Gmail has it. Archive is taking the `INBOX` label away, as Gmail's
//! own archive is; delete is Gmail's own delete too, a move to Trash (emptied
//! after 30 days). Nothing is deleted for good: that needs the full-mail
//! scope.
use super::wire::Thread;
use super::worker::Ask;
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
}

impl InboxPane {
    /// `e`, `d`, `m`, `s` or `u` on the thread under the cursor (or open):
    /// true when it was one of them.
    pub(crate) fn triage(&mut self, ch: &str) -> bool {
        match ch {
            "e" => self.archive(),
            "d" => self.delete(),
            "m" => self.flip("UNREAD", "read", "unread", |t| &mut t.unread),
            "s" => self.flip("STARRED", "unstarred", "starred", |t| &mut t.starred),
            "u" => self.undo(),
            _ => return false,
        }
        true
    }

    /// The thread the keys act on: the open one, else the selected row.
    fn target(&self) -> Option<(usize, Thread)> {
        let id = match &self.reading {
            Some(r) => r.thread.id.clone(),
            None => self.rows.get(self.sel)?.id.clone(),
        };
        let at = self.rows.iter().position(|t| t.id == id)?;
        Some((at, self.rows[at].clone()))
    }

    fn archive(&mut self) {
        let Some((at, thread)) = self.take_row() else {
            return;
        };
        self.relabel(&thread.id, vec![], vec!["INBOX"]);
        self.said(&thread, "archived");
        self.undo = Some(Undo {
            thread,
            at,
            add: vec!["INBOX"],
            remove: vec![],
            returns: true,
            trashed: false,
        });
    }

    fn delete(&mut self) {
        let Some((at, thread)) = self.take_row() else {
            return;
        };
        self.trash(&thread.id, false);
        self.said(&thread, "deleted");
        self.undo = Some(Undo {
            thread,
            at,
            add: vec![],
            remove: vec![],
            returns: true,
            trashed: true,
        });
    }

    /// The target out of the list, and out of the reader if it was open.
    fn take_row(&mut self) -> Option<(usize, Thread)> {
        let (at, thread) = self.target()?;
        self.rows.remove(at);
        self.sel = self.sel.min(self.rows.len().saturating_sub(1));
        self.reading = None;
        Some((at, thread))
    }

    /// Toggle `label` through `field`: `was`/`now` name the two states.
    fn flip(
        &mut self,
        label: &'static str,
        off: &str,
        on: &str,
        field: fn(&mut Thread) -> &mut bool,
    ) {
        let Some((at, before)) = self.target() else {
            return;
        };
        let row = &mut self.rows[at];
        let set = !*field(row);
        *field(row) = set;
        if let Some(r) = self.reading.as_mut() {
            *field(&mut r.thread) = set;
        }
        let (add, remove) = if set {
            (vec![label], vec![])
        } else {
            (vec![], vec![label])
        };
        self.relabel(&before.id, add.clone(), remove.clone());
        self.said(&before, if set { on } else { off });
        self.undo = Some(Undo {
            thread: before,
            at,
            add: remove,
            remove: add,
            returns: false,
            trashed: false,
        });
    }

    fn undo(&mut self) {
        let Some(u) = self.undo.take() else {
            self.note = "nothing to undo".into();
            return;
        };
        match u.returns {
            true => {
                let at = u.at.min(self.rows.len());
                self.rows.insert(at, u.thread.clone());
                self.sel = at;
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

    /// Mark a thread read as it is opened, as Gmail does.
    pub(crate) fn opened(&mut self, id: &str) {
        if let Some(row) = self.rows.iter_mut().find(|t| t.id == id && t.unread) {
            row.unread = false;
            self.relabel(id, vec![], vec!["UNREAD"]);
        }
    }

    fn relabel(&mut self, id: &str, add: Vec<&'static str>, remove: Vec<&'static str>) {
        self.file(Ask::Relabel {
            id: id.into(),
            add,
            remove,
        });
    }

    /// To Trash, or (`back`) out of it.
    fn trash(&mut self, id: &str, back: bool) {
        self.file(Ask::Trash {
            id: id.into(),
            back,
        });
    }

    fn file(&mut self, ask: Ask) {
        let busy = self.busy;
        self.ask(ask);
        self.busy = busy; // filing is not a load: the list stays live
    }

    fn said(&mut self, t: &Thread, what: &str) {
        let subject = match t.subject.trim() {
            "" => "(no subject)",
            s => s,
        };
        self.note = format!("{what} \u{201c}{subject}\u{201d} \u{b7} u undoes");
    }
}

#[cfg(test)]
#[path = "triage_tests.rs"]
mod tests;
