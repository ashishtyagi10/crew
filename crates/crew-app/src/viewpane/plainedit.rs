//! Enter and Tab in a file that is not rendered markdown: a code file, a text file, markdown's
//! own source view. There a newline is a newline carrying the line's indent down, as in every code
//! editor, and Tab is one indent of whatever the file already indents with.
use super::ViewPane;

impl ViewPane {
    /// Enter: a newline, indented as the caret's line is.
    pub(crate) fn newline_plain(&mut self, cols: u16, rows: u16) {
        let Some(at) = self.caret_at else { return };
        let Some(src) = self.source_str() else { return };
        let at = (at as usize).min(src.len());
        let line = src[..at].rfind('\n').map_or(0, |i| i + 1);
        let indent: String = src[line..at]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        // A CRLF file stays one: the line split here keeps its `\r`, and so does the new one.
        let nl = if src.contains("\r\n") { "\r\n" } else { "\n" };
        self.insert(&format!("{nl}{indent}"), cols, rows);
    }

    /// Tab: a tab in a file indented with tabs, four spaces otherwise.
    pub(crate) fn tab_plain(&mut self, cols: u16, rows: u16) {
        let tabs = self
            .source_str()
            .is_some_and(|s| s.starts_with('\t') || s.contains("\n\t"));
        self.insert(if tabs { "\t" } else { "    " }, cols, rows);
    }
}
