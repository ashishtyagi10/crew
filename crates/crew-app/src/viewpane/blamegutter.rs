//! Laying the blame column down beside lines that are already rendered.
//!
//! The gutter rungs all write the same thing in their first
//! [`super::lines::GUTTER_W`] columns: a right-aligned source line number on
//! the row a line starts, and a `↪` continuation mark on every row it wrapped
//! onto. That is a complete answer to "which source line is this row", so the
//! blame column is prepended to the finished lines rather than threaded
//! through every rung — the same move [`crate::diffrefine`] makes when it
//! reads a rendering back to find out which side of a diff a row is.
//!
//! The rendering it is prepended to must already have been laid out narrower
//! by exactly this width. Prepending to a full-width rendering would push
//! every row past the pane's right edge, which is why [`apply`] is called
//! with the width the caller subtracted, not one it chooses for itself.
use crate::chatbody::{plain, CardLine};

use super::lines::GUTTER_W;

/// The source line (1-based) a rendered `line` belongs to, or `None` for a
/// wrap continuation, a banner, or anything else without a numbered gutter.
fn source_line(line: &CardLine) -> Option<usize> {
    let head: String = line.iter().take(GUTTER_W).map(|c| c.c).collect();
    head.trim().parse::<usize>().ok()
}

/// Prepend `labels[n - 1]` to each row, in `width` columns. Rows with no
/// source line of their own — wrap continuations, banners — get blanks, so
/// the text column stays exactly where it is on every row.
pub(crate) fn apply(lines: &mut [CardLine], labels: &[String], width: usize) {
    if width == 0 {
        return;
    }
    let fg = crew_theme::theme().text_muted;
    let blank = " ".repeat(width);
    for line in lines.iter_mut() {
        let label = source_line(line)
            .and_then(|n| labels.get(n - 1))
            .map_or(blank.as_str(), String::as_str);
        // Pad here as well as in `labels`: a short label (or none at all, on
        // a row past the end of the blame) must still occupy its column, or
        // the code beside it steps left and the file reads as ragged.
        let mut head: CardLine = label
            .chars()
            .chain(std::iter::repeat(' '))
            .take(width)
            .map(|c| plain(c, fg, false))
            .collect();
        head.append(&mut std::mem::take(line));
        *line = head;
    }
}

#[cfg(test)]
#[path = "blamegutter_tests.rs"]
mod tests;
