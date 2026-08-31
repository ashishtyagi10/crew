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
fn clip_w_marks_the_cut_on_a_cell_boundary() {
    assert_eq!(clip_w("abcdefgh", 5), "abcd…");
    assert_eq!(clip_w("fits", 5), "fits");
    // Wide glyphs never split: clipping "日本語" at 5 keeps whole chars.
    let wide = clip_w("日本語", 5);
    assert!(str_w(&wide) <= 5);
    assert!(wide.ends_with('…'));
    // A zero budget yields nothing at all — not an overflowing '…'.
    assert_eq!(clip_w("abc", 0), "");
}

#[test]
fn fit_end_always_advances() {
    let wide: Vec<char> = "\u{4e2d}".chars().collect();
    // A 2-wide glyph in a 1-column budget still advances past it.
    assert_eq!(fit_end(&wide, 0, 1), 1);
    assert_eq!(fit_end(&wide, 1, 1), 1, "at the end it stays put");
}
