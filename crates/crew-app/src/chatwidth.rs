//! Display-width helpers for the chat views. The cell grid is width-aware
//! (a wide emoji/CJK glyph occupies two cells — its advance snaps to
//! 2 × cell at render), so wrapping and column placement must count display
//! columns, not chars, or text after a wide glyph overlaps it.
use unicode_width::UnicodeWidthChar;

/// Display columns `c` occupies in the cell grid (0 for zero-width marks).
pub(crate) fn char_w(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// The furthest `end` such that `full[start..end]` fits `cols` display
/// columns. Always advances at least one char when any remain, so wrapping
/// loops can never stall on an over-wide glyph.
pub(crate) fn fit_end(full: &[char], start: usize, cols: usize) -> usize {
    let mut w = 0;
    let mut end = start;
    while end < full.len() {
        let cw = char_w(full[end]);
        if w + cw > cols {
            break;
        }
        w += cw;
        end += 1;
    }
    if end == start && start < full.len() {
        start + 1
    } else {
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_one_column_wide_glyphs_two() {
        assert_eq!(char_w('a'), 1);
        assert_eq!(char_w('\u{4e2d}'), 2); // 中
        assert_eq!(char_w('\u{1f600}'), 2); // 😀
        assert_eq!(char_w('\u{200d}'), 0); // zero-width joiner
    }

    #[test]
    fn fit_end_counts_display_columns() {
        let ascii: Vec<char> = "abcdef".chars().collect();
        assert_eq!(fit_end(&ascii, 0, 4), 4);
        assert_eq!(fit_end(&ascii, 4, 4), 6);
        // Three wide glyphs: only two fit in 5 columns.
        let wide: Vec<char> = "\u{4e2d}\u{4e2d}\u{4e2d}".chars().collect();
        assert_eq!(fit_end(&wide, 0, 5), 2);
    }

    #[test]
    fn fit_end_always_advances() {
        let wide: Vec<char> = "\u{4e2d}".chars().collect();
        // A 2-wide glyph in a 1-column budget still advances past it.
        assert_eq!(fit_end(&wide, 0, 1), 1);
        assert_eq!(fit_end(&wide, 1, 1), 1, "at the end it stays put");
    }
}
