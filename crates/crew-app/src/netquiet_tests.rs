//! A NET chart with nothing to draw says so.
use super::*;
use crate::nettwin::{reading, Reading, FLOOR};

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn a_quiet_chart_is_captioned_on_its_upper_row() {
    let _g = crate::app::theme_test_guard();
    let quiet = Reading {
        ceiling: FLOOR,
        quiet: true,
    };
    assert_eq!(row_text(&net_cells(0, 0, quiet, 28), 2), "no traffic");
    assert!(
        row_text(&net_cells(0, 0, FLOOR, 28), 2).is_empty(),
        "a plain ceiling says nothing"
    );
    // Too narrow for the words: nothing, rather than a clipped caption.
    assert!(row_text(&net_cells(0, 0, quiet, 12), 2).is_empty());
}

#[test]
fn quiet_is_both_directions_under_a_kilobyte_across_the_chart() {
    let mut rx = crate::spark::History::new(120);
    let mut tx = crate::spark::History::new(120);
    for _ in 0..40 {
        rx.push(300);
        tx.push(80);
    }
    assert!(reading(&rx, &tx, 28).quiet, "background chatter is quiet");
    rx.push(40_000);
    assert!(!reading(&rx, &tx, 28).quiet, "a burst on screen is not");
}
