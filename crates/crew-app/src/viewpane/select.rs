//! Selecting a run of the document, and the two commands that need one.
//!
//! A selection is a pair of source bytes — where the selecting began and
//! where the caret is now — because that is the only description of it that
//! survives the document being re-wrapped, and the only one an edit can act
//! on. What gets washed on screen falls out of it: a cell is selected when
//! the byte it came from is inside the range.
//!
//! **Bold and italic are why this exists.** The promise is that no `**` ever
//! appears on screen, which means there has to be some other way to put one
//! in the file — and "wrap what is selected" is that way. It is also the one
//! place where an edit is spelled in markdown rather than in prose, so it is
//! kept in one small file where that is obvious.
use super::ViewPane;

/// The selected byte range, low to high, or `None` when nothing is selected
/// (or the selection is empty, which is the same thing to everything that
/// reads it).
pub(crate) fn range(p: &ViewPane) -> Option<(u32, u32)> {
    let (a, b) = (p.anchor?, p.caret_at?);
    let (lo, hi) = (a.min(b), a.max(b));
    (lo < hi).then_some((lo, hi))
}

impl ViewPane {
    /// Begin selecting, if nothing is selected yet: the anchor is where the
    /// caret is now, and the arrow that follows moves the other end.
    pub(crate) fn anchor_here(&mut self) {
        if self.anchor.is_none() {
            self.anchor = self.caret_at;
        }
    }

    pub(crate) fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Select the whole document.
    pub(crate) fn select_all(&mut self, cols: u16, rows: u16) {
        let Some(end) = self.source_len() else { return };
        self.anchor = Some(0);
        self.caret_at = Some(end);
        self.relayout_caret(cols, rows);
    }

    /// The selected source text, for the clipboard. The SOURCE, not the
    /// render: what is copied out of a markdown document and pasted into
    /// another one has to still be markdown.
    pub(crate) fn selected_text(&self) -> Option<String> {
        let (lo, hi) = range(self)?;
        let src = self.source_str()?;
        src.get(lo as usize..hi as usize).map(str::to_string)
    }

    /// Wrap the selection in `marker` — or take the marker off again when it
    /// is already there, which is what makes the key a toggle rather than a
    /// way to accumulate asterisks.
    ///
    /// `false` when the selection spans more than one block, where there is
    /// no honest answer: emphasis is an INLINE thing, so a `**` opened in a
    /// heading has no partner in the paragraph below it and markdown renders
    /// it as two asterisks — the one thing this editor promises never to put
    /// on your screen. (Found by looking at a shot of exactly that.)
    pub(crate) fn wrap_selection(&mut self, marker: &str, cols: u16, rows: u16) -> bool {
        let Some((lo, hi)) = range(self) else {
            return false;
        };
        let Some(src) = self.source_str() else {
            return false;
        };
        if crosses_block(src, lo, hi) {
            return false;
        }
        // A delimiter has to FLANK the text it emphasizes: markdown will not
        // read `**word **` as bold, because the closing pair is preceded by a
        // space, and renders the asterisks literally instead. So the range is
        // trimmed to what it is actually emphasizing — which is also what
        // every other editor does when you select a word and its trailing
        // space. (The second bug a shot of this caught.)
        let (lo, hi) = match trimmed(src, lo, hi) {
            Some(r) => r,
            None => return false,
        };
        let n = marker.len() as u32;
        let wrapped = lo >= n
            && src
                .get((lo - n) as usize..lo as usize)
                .is_some_and(|s| s == marker)
            && src
                .get(hi as usize..(hi + n) as usize)
                .is_some_and(|s| s == marker);
        match wrapped {
            // Take it off from the far end first: removing the leading marker
            // would move every byte after it, including the trailing one.
            true => {
                self.replace_range(hi, hi + n, "", cols, rows);
                self.replace_range(lo - n, lo, "", cols, rows);
                self.anchor = Some(lo - n);
                self.caret_at = Some(hi - n);
            }
            false => {
                self.replace_range(hi, hi, marker, cols, rows);
                self.replace_range(lo, lo, marker, cols, rows);
                self.anchor = Some(lo + n);
                self.caret_at = Some(hi + n);
            }
        }
        self.relayout_caret(cols, rows);
        true
    }

    /// Delete what is selected, leaving the caret where it began.
    pub(crate) fn delete_selection(&mut self, cols: u16, rows: u16) -> bool {
        let Some((lo, hi)) = range(self) else {
            return false;
        };
        self.replace_range(lo, hi, "", cols, rows);
        self.anchor = None;
        self.caret_at = Some(lo);
        self.relayout_caret(cols, rows);
        true
    }
}

/// `[lo, hi)` with leading and trailing whitespace taken off, or `None` when
/// there is nothing but whitespace in it.
fn trimmed(src: &str, lo: u32, hi: u32) -> Option<(u32, u32)> {
    let text = src.get(lo as usize..hi as usize)?;
    let lead = text.len() - text.trim_start().len();
    let tail = text.len() - text.trim_end().len();
    let (a, b) = (lo + lead as u32, hi - tail as u32);
    (a < b).then_some((a, b))
}

/// Whether `[lo, hi)` reaches out of the block it started in — a blank line,
/// or a line that opens a construct of its own.
fn crosses_block(src: &str, lo: u32, hi: u32) -> bool {
    let Some(text) = src.get(lo as usize..hi as usize) else {
        return true;
    };
    for line in text.split('\n').skip(1) {
        let t = line.trim_start();
        if t.is_empty() {
            return true;
        }
        if t.starts_with('#')
            || t.starts_with('>')
            || t.starts_with('|')
            || t.starts_with("```")
            || ["- ", "* ", "+ "].iter().any(|m| t.starts_with(m))
        {
            return true;
        }
        let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            let after = &t[digits.len()..];
            if after.starts_with(". ") || after.starts_with(") ") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
