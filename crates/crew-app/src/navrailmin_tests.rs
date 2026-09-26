//! A pane off the grid says so on the rail too.
use super::*;

fn pane(index: usize, minimized: bool) -> PaneRow {
    PaneRow {
        index,
        title: format!("pane {index}"),
        focused: false,
        activity: false,
        minimized,
        attention: None,
        busy: false,
        unread: 0,
        hovered: false,
    }
}

fn mark(v: &[CellView], row: u16) -> Option<char> {
    v.iter()
        .find(|c| c.col == RAIL_COLS - 1 && c.row == row)
        .map(|c| c.c)
}

/// The full list's `+`, in the rail's mark column; the attention glyph still
/// outranks it.
#[test]
fn a_minimized_pane_wears_its_plus_on_the_rail() {
    let _g = crate::app::theme_test_guard();
    let mut waiting = pane(3, true);
    waiting.attention = Some(('!', true));
    let rows = [pane(1, false), pane(2, true), waiting];
    let v = rail_cells(&rows, RAIL_COLS, 8, '*');
    assert_eq!(mark(&v, 0), None);
    assert_eq!(mark(&v, 1), Some('+'));
    assert_eq!(mark(&v, 2), Some('!'), "needs-you wins the one slot");
}
