use super::{hint, HINTS};

#[test]
fn every_form_says_esc_and_the_ladder_is_longest_first() {
    let w: Vec<usize> = HINTS.iter().map(|s| crate::chatwidth::str_w(s)).collect();
    assert!(w.windows(2).all(|p| p[0] > p[1]), "{w:?}");
    assert!(HINTS.iter().all(|s| s.contains("esc")));
}

#[test]
fn the_widest_form_that_fits_is_chosen() {
    assert_eq!(hint(80), HINTS[0]);
    assert_eq!(hint(30), HINTS[2]);
    assert_eq!(hint(20), HINTS[3]);
    assert_eq!(hint(5), HINTS[3], "never nothing");
}

/// The pane's last row carries the chosen form whole: no `…`, no cut.
#[test]
fn the_pane_draws_the_hint_whole_at_its_narrowest() {
    let _g = crate::app::theme_test_guard();
    let p = crate::diskpane::DiskPane::new(std::env::temp_dir());
    for cols in [20u16, 30, 47, 80] {
        let cells = p.cells(cols, 12);
        let mut last: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == 11).collect();
        last.sort_by_key(|c| c.col);
        let s: String = last.iter().map(|c| c.c).collect();
        assert_eq!(s.trim_end(), hint(cols), "{cols} cols");
        assert!(!s.contains('\u{2026}'), "{cols} cols: {s:?}");
    }
}
