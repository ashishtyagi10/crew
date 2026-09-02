//! What Cmd+S and Cmd+R do to a document window, after the key is classified.
//! Split from [`super::event`], which routes; this decides.

/// Whether a re-read may go ahead: an editor with unsaved changes asks once,
/// the same once Esc asks, and the second press discards. `Err` is what to
/// say while asking.
pub(crate) fn guard(dirty: bool, warned: bool) -> Result<(), &'static str> {
    match dirty && !warned {
        true => Err("unsaved changes \u{2014} Cmd+S to save, Cmd+R again to re-read the file and discard them"),
        false => Ok(()),
    }
}

impl crate::app::CrewApp {
    /// Cmd+S: write the document, and say so on the status line.
    pub(crate) fn save_doc(&mut self, i: usize) {
        let d = &mut self.docs[i];
        let name = d.view.path.display().to_string();
        d.warned = false;
        match d.view.save() {
            Ok(()) => self.set_status(format!("saved {name}")),
            Err(e) => self.set_status(format!("could not save {name}: {e}")),
        }
    }

    /// Cmd+R: re-read the file from disk — an agent may have written it
    /// underneath you — dropping unsaved edits only after asking once.
    pub(crate) fn reread_doc(&mut self, i: usize) {
        let d = &mut self.docs[i];
        if let Err(ask) = guard(d.view.dirty, d.warned) {
            d.warned = true;
            self.set_status(ask);
            return;
        }
        d.warned = false;
        let name = d.view.path.display().to_string();
        d.view.reread();
        self.set_status(format!("re-reading {name}"));
    }
}

#[cfg(test)]
#[path = "reread_tests.rs"]
mod tests;
