use super::{stamp, text, FORMS};
use crew_render::CellView;

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn every_form_names_esc_and_the_widest_that_fits_is_chosen() {
    assert!(FORMS.iter().all(|s| s.contains("esc")));
    assert_eq!(text(80), FORMS[0]);
    assert_eq!(text(26), FORMS[1]);
    assert_eq!(text(24), FORMS[2]);
    assert_eq!(text(8), FORMS[3]);
}

#[test]
fn the_legend_replaces_the_last_row_and_fits_it() {
    let _g = crate::app::theme_test_guard();
    let mut cells: Vec<CellView> = (0..10)
        .map(|col| CellView {
            col,
            row: 5,
            c: 'x',
            ..Default::default()
        })
        .collect();
    stamp(&mut cells, 40, 6);
    let last = row_text(&cells, 5);
    assert_eq!(last.trim_end(), FORMS[0], "{last:?}");
    assert!(!last.contains('x'), "the old row is gone: {last:?}");
    assert!(cells.iter().all(|c| c.col < 40));
    // Too short to give a row up: untouched.
    let mut one = vec![CellView {
        c: 'x',
        ..Default::default()
    }];
    stamp(&mut one, 40, 1);
    assert_eq!(one.len(), 1);
}

/// A pane hint mode is not on keeps every cell it had.
#[test]
fn an_unlabelled_pane_is_left_alone() {
    let _g = crate::app::theme_test_guard();
    let mut cells = vec![CellView {
        row: 3,
        c: 'x',
        ..Default::default()
    }];
    super::mark(&mut cells, 999, 40, 4);
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].c, 'x');
}
