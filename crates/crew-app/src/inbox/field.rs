//! One place to type in the reply box: the `to` line, the `cc` line, or the
//! words. Text and a caret counted in chars; a line (`to`, `cc`) holds no
//! line break, so a pasted list of addresses joins with commas.

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Field {
    pub text: String,
    /// The caret, in chars.
    pub at: usize,
}

impl Field {
    pub(crate) fn with(text: &str) -> Self {
        Self {
            text: text.into(),
            at: text.chars().count(),
        }
    }

    fn byte(&self, at: usize) -> usize {
        self.text
            .char_indices()
            .nth(at)
            .map_or(self.text.len(), |(b, _)| b)
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// `text` at the caret. `line`: one line, breaks made `, `.
    pub(crate) fn put(&mut self, text: &str, line: bool) {
        let unix = text.replace("\r\n", "\n");
        let joined = match line {
            true => unix.trim_end_matches('\n').replace('\n', ", "),
            false => unix,
        };
        let clean: String = joined
            .chars()
            .filter(|&ch| ch == '\n' || !ch.is_control())
            .collect();
        let b = self.byte(self.at);
        self.text.insert_str(b, &clean);
        self.at += clean.chars().count();
    }

    pub(crate) fn back(&mut self) {
        if self.at > 0 {
            self.at -= 1;
            let b = self.byte(self.at);
            self.text.remove(b);
        }
    }

    pub(crate) fn delete(&mut self) {
        if self.at < self.len() {
            let b = self.byte(self.at);
            self.text.remove(b);
        }
    }

    pub(crate) fn left(&mut self) {
        self.at = self.at.saturating_sub(1);
    }

    pub(crate) fn right(&mut self) {
        self.at = (self.at + 1).min(self.len());
    }

    /// Home and End: the start and end of the caret's own line.
    pub(crate) fn home(&mut self) {
        let (line, _) = self.place();
        self.at = self.starts()[line];
    }

    pub(crate) fn end(&mut self) {
        let (line, _) = self.place();
        self.at = self.starts().get(line + 1).map_or(self.len(), |s| s - 1);
    }

    /// The caret a line up (`up`) or down, at the same column or the line's
    /// end; `false` when there is no line there, so the caret may leave.
    pub(crate) fn line(&mut self, up: bool) -> bool {
        let (line, col) = self.place();
        let starts = self.starts();
        let to = match up {
            true if line == 0 => return false,
            true => line - 1,
            false if line + 1 >= starts.len() => return false,
            false => line + 1,
        };
        let end = starts.get(to + 1).map_or(self.len(), |s| s - 1);
        self.at = (starts[to] + col).min(end);
        true
    }

    /// Where each line starts, in chars.
    fn starts(&self) -> Vec<usize> {
        let mut out = vec![0];
        out.extend(
            self.text
                .chars()
                .enumerate()
                .filter(|&(_, c)| c == '\n')
                .map(|(i, _)| i + 1),
        );
        out
    }

    /// The caret's line, and its column in that line.
    fn place(&self) -> (usize, usize) {
        let starts = self.starts();
        let line = starts.iter().rposition(|&s| s <= self.at).unwrap_or(0);
        (line, self.at - starts[line])
    }
}
