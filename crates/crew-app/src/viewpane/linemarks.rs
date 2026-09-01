//! Two paints that mark rather than colour: a diff's added and removed sides,
//! and the dots on trailing whitespace.
//!
//! Split from [`super::linepaint`] for the line cap.
use super::linepaint::*;
use crate::chatbody::CardLine;
use crate::viewpane::outline::Mark;

/// Trailing whitespace on an ADDED line, shown as middle dots.
///
/// It is the review nit every diff tool marks, because it is invisible by
/// construction: the reviewer cannot see it and the author did not mean it.
/// Only added lines — what a removed line trailed with is not news — and only
/// past the marker column, so a line of pure indentation still reads as one.
pub(crate) fn mark_trailing_space(line: &mut CardLine, added: bool) {
    if !added {
        return;
    }
    let fg = crew_theme::theme().bell;
    let start = line
        .iter()
        .rposition(|c| !c.c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    // `GUTTER_W + 1`: the gutter and the `+` marker are not the line's text.
    for cell in line.iter_mut().skip(start.max(GUTTER_W + 1)) {
        if cell.c == ' ' {
            cell.c = '\u{b7}';
            cell.fg = fg;
        }
    }
}

/// The diff rung: a review rather than a colour per line. Pairing, word-level
/// marks and the header treatment live in [`super::diffpaint`]; this only lays
/// that paint down through the same numbered gutter every other rung uses.
pub(crate) fn diff_lines(text: &str, cols: usize, ws: &[Vec<bool>]) -> (Vec<CardLine>, Vec<Mark>) {
    let t = crew_theme::theme();
    let mut paints = super::diffpaint::paint(text);
    super::whitespace::dim(&mut paints, ws, t.text_muted);
    let (mut lines, src) = painted(text, cols, &paints, t.ink, t.text_muted);
    // The gutter says where in the SOURCE you are, not where in the patch —
    // the same numbers the side-by-side rung has always shown (`diffnums`).
    renumber(
        &mut lines,
        &src,
        &super::diffnums::numbers(text),
        t.text_muted,
    );
    // Only the row a source line STARTS on carries its marker, so only that
    // row can be an added line whose tail is worth marking.
    let kinds: Vec<super::diffpaint::Kind> =
        text.split('\n').map(super::diffpaint::kind_of).collect();
    let mut last = usize::MAX;
    for (row, line) in lines.iter_mut().enumerate() {
        let n = src.get(row).copied().unwrap_or(0);
        let first = n != last;
        last = n;
        let added = first && kinds.get(n) == Some(&super::diffpaint::Kind::Added);
        mark_trailing_space(line, added);
    }
    // Landmarks are found in the source and reported as ROWS: a wrapped line
    // occupies several, and `]` has to land on the first of them.
    let marks = super::outline::diff_marks(text)
        .into_iter()
        .filter_map(|(line, label)| {
            let row = src.iter().position(|&s| s == line)?;
            Some(Mark {
                row,
                label,
                depth: 0,
            })
        })
        .collect();
    (lines, marks)
}
