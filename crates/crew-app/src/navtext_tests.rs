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
