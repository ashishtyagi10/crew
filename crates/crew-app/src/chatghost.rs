//! What the crew composer offers while you type: the ghost completion sitting
//! past the cursor, accepting it, and keeping the command palette in step with
//! the text.
//!
//! Split from [`crate::chattype`] for the line cap, along the line between
//! what a key DOES and what the composer SUGGESTS.
use crate::chat::ChatPane;

impl ChatPane {
    /// The rest of a previous prompt that starts with what is typed, shown
    /// dim after the caret and taken with Tab or Right.
    ///
    /// `suggest` is the input bar's rule, called with this pane's own history
    /// — the third time a composer behaviour has turned out to exist already
    /// (Up/Down recall, prefix search, now this). A leading `/` belongs to the
    /// palette, which is showing the same list as a popup and would be
    /// answering the same question twice.
    pub(crate) fn ghost(&self) -> Option<String> {
        if self.input.starts_with('/') || self.input.contains('\n') {
            return None;
        }
        crate::suggest::suggest(&self.input, self.history.lines())
    }

    /// Take the whole suggestion. Nothing to take is not an error — Tab and
    /// Right are pressed speculatively.
    pub(crate) fn accept_ghost(&mut self) {
        if let Some(g) = self.ghost() {
            self.input.push_str(&g);
            self.history.edited(); // the text is the user's now
        }
    }

    /// Re-sync the leading-token palette to the current input. Called after a
    /// character edit and after a palette row is accepted (`PaletteKey::
    /// Accepted`) — both change the input, and the palette must follow it.
    pub(crate) fn sync_palette(&mut self, cwd: &std::path::Path) {
        let agents = self.agents.clone();
        let current = crate::chatpalette::shared_model(&self.agents);
        crate::chatpalette::after_edit(&mut self.palette, &self.input, current.as_deref(), || {
            crate::chatmention::scan_entries(cwd, &agents)
        });
    }
}
