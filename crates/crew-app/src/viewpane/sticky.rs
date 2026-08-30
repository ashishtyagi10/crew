//! The heading you are underneath, kept on the top row.
//!
//! A document is read in sections, and the moment a section is longer than the
//! window the one thing the window stops telling you is which section you are
//! in. The card's gutter marks *where* the headings are — it always did — and
//! that answers "how far", never "under what". Scroll into the middle of a
//! long spec and the pane is prose with no address.
//!
//! So the heading above the top row is drawn ON the top row: a band in the
//! page's own hand, one line, with the ladder of headings it sits under
//! collapsed into it (`Themes \u{203a} CRT`), so the address is complete rather
//! than merely nearest. It costs a row of the document and returns the
//! question that row was raising.
//!
//! Nothing sticks when the heading is already on screen — a title repeated one
//! row above itself is noise — and nothing sticks at the top of the file,
//! where the document's own first line is the address.
use crew_render::CellView;

use super::outline::Mark;

/// The heading text for a document scrolled to `top`, or `None` when the
/// heading above is already visible (or there is none).
///
/// `depth_of` says how deep a mark is, so an inner heading can be shown under
/// its parents. Marks with no depth (a diff's file/hunk rows) all read as one
/// level, which is right: a hunk is not inside the file header, it is after it.
pub(crate) fn label_for(marks: &[Mark], top: usize) -> Option<String> {
    if top == 0 {
        return None;
    }
    // The last mark at or above the top row is the section we are inside.
    let here = marks.iter().rfind(|m| m.row <= top)?;
    // Already on screen: `row == top` means the heading IS the top row.
    if here.row == top {
        return None;
    }
    // Walk back up the ladder: each ancestor is the nearest earlier mark
    // shallower than the one before it. A landmark with no depth (a diff's
    // rows) has no ladder, so the trail is just itself.
    let mut trail: Vec<&str> = vec![here.label.as_str()];
    let mut want = here.depth;
    if want > 0 {
        for m in marks.iter().rev().skip_while(|m| m.row >= here.row) {
            if m.depth > 0 && m.depth < want {
                trail.push(m.label.as_str());
                want = m.depth;
                if want == 1 {
                    break;
                }
            }
        }
    }
    trail.reverse();
    Some(trail.join(" \u{203a} "))
}

/// Draw the band over row 0 of `cells`, replacing whatever was there.
pub(crate) fn draw(cells: &mut Vec<CellView>, label: &str, cols: u16) {
    if cols == 0 || label.is_empty() {
        return;
    }
    let t = crew_theme::theme();
    let bg = crate::chatink::code_bg();
    cells.retain(|c| c.row != 0);
    // The band spans the full width even where the label does not, or it
    // reads as a highlighted phrase rather than as a rule the page hangs off.
    for col in 0..cols {
        cells.push(CellView {
            col,
            row: 0,
            c: ' ',
            fg: t.text_muted,
            bg,
            ..Default::default()
        });
    }
    let text = format!(" {label}");
    crate::chatwidth::place_row(
        0,
        cols,
        text.chars().map(|c| (c, t.text_muted)),
        |col, c, fg| {
            cells.retain(|x| !(x.row == 0 && x.col == col));
            cells.push(CellView {
                col,
                row: 0,
                c,
                fg,
                bg,
                bold: true,
                ..Default::default()
            });
        },
    );
}

#[cfg(test)]
#[path = "sticky_tests.rs"]
mod tests;
