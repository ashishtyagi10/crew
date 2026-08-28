//! Text placement shared by the left nav's sections, and the rule they all
//! degrade by.
//!
//! Every section drew its own copy of "write at column 3, `.take(cols - 4)`",
//! which is a *character* clip: at the narrow end of the resize edge's range
//! the nav showed `Mac.lan · Darw`, `↑ 0 B` and a load average of `3.` — and a
//! half-written number is not a smaller reading, it is a wrong one.
//!
//! Two rules, then:
//!
//! - **Prose ellipsizes.** A host name, a pane title, a log line: the reader
//!   can see it was cut, and the head still says which one it is.
//! - **A row of values drops whole values.** [`fit`] picks the widest form
//!   that fits from a ladder the section writes longest-first, so a narrow nav
//!   shows two load averages rather than two and a half.
use crew_render::CellView;

/// Column the nav's section content starts on, aligned under the rule's legend.
pub const INDENT: u16 = 3;

/// Display columns a section's content row has at `cols` wide: the indent, and
/// one column of air before the card's right border.
pub fn budget(cols: u16) -> usize {
    cols.saturating_sub(INDENT + 1) as usize
}

/// The first form in `ladder` that fits `cols`, or the last one clipped —
/// written longest-first. For a row of values, where cutting one in half is
/// worse than not showing it.
pub fn fit<'a>(ladder: &[&'a str], cols: u16) -> &'a str {
    let room = budget(cols);
    ladder
        .iter()
        .find(|s| crate::chatwidth::str_w(s) <= room)
        .or_else(|| ladder.last())
        .copied()
        .unwrap_or("")
}

/// Write `s` at the nav's indent on `row`, ellipsized to the row's budget.
pub fn put(out: &mut Vec<CellView>, s: &str, row: u16, cols: u16, fg: (u8, u8, u8)) {
    put_at(out, s, INDENT, row, cols.saturating_sub(1), fg);
}

/// [`put`] with the column and right edge given: for a section laying out more
/// than one run on a row.
pub fn put_at(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    max_col: u16,
    fg: (u8, u8, u8),
) {
    let bg = crew_theme::theme().page_bg;
    let clipped = crate::chatwidth::clip_w(s, max_col.saturating_sub(col) as usize);
    crate::chatwidth::place_row(
        col,
        max_col,
        clipped.chars().map(|c| (c, fg)),
        |x, c, fg| {
            out.push(CellView {
                col: x,
                row,
                c,
                fg,
                bg,
                ..Default::default()
            })
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point: a ladder gives up a value, never half of one.
    #[test]
    fn fit_takes_the_widest_form_that_fits() {
        let ladder = ["4.51  4.20  3.64", "4.51 4.20 3.64", "4.51 4.20", "4.51"];
        assert_eq!(fit(&ladder, 24), "4.51  4.20  3.64"); // budget 20
        assert_eq!(fit(&ladder, 19), "4.51 4.20 3.64"); // budget 15
        assert_eq!(fit(&ladder, 17), "4.51 4.20"); // budget 13
        assert_eq!(fit(&ladder, 12), "4.51"); // budget 8
                                              // Narrower than anything on the ladder: the shortest form, which the
                                              // write then clips — but the section has already given up everything
                                              // it could give up first.
        assert_eq!(fit(&ladder, 4), "4.51");
    }

    #[test]
    fn put_ellipsizes_rather_than_cutting_mid_word() {
        let _g = crate::app::theme_test_guard();
        let mut out = Vec::new();
        put(&mut out, "Mac.lan · Darwin", 1, 18, (1, 2, 3));
        let text: String = {
            out.sort_by_key(|c| c.col);
            out.iter().map(|c| c.c).collect()
        };
        assert!(text.ends_with('…'), "{text:?}");
        assert_eq!(crate::chatwidth::str_w(&text), budget(18));
        assert_eq!(out[0].col, INDENT);
    }

    #[test]
    fn a_row_that_fits_is_untouched() {
        let _g = crate::app::theme_test_guard();
        let mut out = Vec::new();
        put(&mut out, "up 2h 6m", 1, 24, (1, 2, 3));
        let text: String = out.iter().map(|c| c.c).collect();
        assert_eq!(text, "up 2h 6m");
    }
}
