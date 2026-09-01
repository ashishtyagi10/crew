//! Asking the viewer where things are: the scroll position it reports, which
//! rows carry marks, and which rows a hit-test should consider.
//!
//! Split from [`super::render`] for the line cap.
use crate::viewpane::ViewPane;

impl ViewPane {
    /// `(lines back from the bottom, total lines)` — the terminal-shaped
    /// numbers the card's scroll thumb is written in. The viewer counts rows
    /// DOWN from the top and a terminal counts them BACK from the live
    /// bottom, so this is where the two meet; getting it backwards draws a
    /// thumb that climbs as you scroll down.
    pub(crate) fn position(&self, cols: u16, rows: u16) -> (usize, usize) {
        if cols == 0 || rows == 0 {
            return (0, 0);
        }
        let total = self.lines_for(cols).lines.len();
        let visible = usize::from(rows);
        let back = total.saturating_sub(visible).saturating_sub(self.scroll);
        (back, total)
    }

    /// Rendered rows worth marking on the card's gutter: headings in a
    /// document, files and hunks in a review.
    pub(crate) fn mark_rows(&self, cols: u16) -> Vec<usize> {
        self.lines_for(cols).marks.iter().map(|m| m.row).collect()
    }

    /// Rendered rows holding a live search hit, for the card's gutter. Empty
    /// when nothing is being searched for.
    pub(crate) fn hit_rows(&self) -> Vec<usize> {
        self.search
            .as_ref()
            .map(|s| s.hits.clone())
            .unwrap_or_default()
    }
}
