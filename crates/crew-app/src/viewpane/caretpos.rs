//! Where the caret is in the FILE. The caret lives in the render, but the
//! number a person quotes to someone else — "line 40, column 12" — is the
//! source's, the same one an external editor shows for the same byte.
use super::ViewPane;

impl ViewPane {
    /// `(line, column)` of the caret's byte, both 1-based, the column in
    /// characters rather than bytes. `None` without a caret or a document.
    pub(crate) fn caret_line_col(&self) -> Option<(usize, usize)> {
        let at = self.caret_at? as usize;
        let src = self.source_str()?;
        let before = src.get(..at.min(src.len()))?;
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        Some((line, col))
    }
}

#[cfg(test)]
#[path = "caretpos_tests.rs"]
mod tests;
