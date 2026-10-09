//! The empty list's hint and the composer's placeholder fit a narrow tile
//! by what they drop — the example, the long form — never mid-word at the
//! edge, and never by losing the way out.
use crate::todopane::test_pane;

fn row_text(cells: &[crew_render::CellView], row: u16) -> String {
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn the_empty_hint_marks_its_cut_and_stays_inside_the_pane() {
    let _g = crate::app::theme_test_guard();
    let p = test_pane(vec![]);
    let cols = 30u16;
    let cells = crate::todopane::render::cells(&p, cols, 12);
    let hint = (0..12)
        .map(|r| row_text(&cells, r))
        .find(|s| s.contains("type one below"))
        .expect("the hint row");
    assert!(!hint.contains("try:"), "the example goes whole: {hint:?}");
    assert!(cells.iter().all(|c| c.col < cols), "{hint:?}");
    let wide = crate::todopane::render::cells(&p, 80, 12);
    let whole = (0..12)
        .map(|r| row_text(&wide, r))
        .find(|s| s.contains("@home"));
    assert!(whole.is_some(), "the whole hint on a wide pane");
}

#[test]
fn the_done_view_placeholder_marks_its_cut() {
    let _g = crate::app::theme_test_guard();
    let mut p = test_pane(vec![]);
    p.done_view = true;
    let cells = crate::todopane::render::cells(&p, 24, 12);
    let ph = (0..12)
        .map(|r| row_text(&cells, r))
        .find(|s| s.contains("filter"))
        .expect("the placeholder row");
    assert!(ph.contains("filter \u{b7} esc leaves"), "{ph:?}");
    assert!(cells.iter().all(|c| c.col < 24), "{ph:?}");
}
