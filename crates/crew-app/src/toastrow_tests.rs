//! Where the toast stack starts. It used to open at the very top of the
//! content area, on the title row of the pane tiled there, and a card sat
//! over that pane's `[-][x]` for as long as it lived: no closing the pane
//! while crew was telling you something. And its right edge sat on that
//! pane's right stroke, the two frames merging into one (2026-10-09 survey).
use crate::layout::Rect;
use crate::toast::{push_toasts, Toasts};

#[test]
fn the_stack_leaves_the_top_panes_title_row_clear() {
    let mut t = Toasts::default();
    t.push("swarm finished".into(), "done", false, 0);
    t.push("cargo test failed".into(), "bell", true, 0);
    let content = Rect {
        x: 0.0,
        y: 40.0,
        w: 800.0,
        h: 600.0,
    };
    let ch = 16.0;
    let mut scenes = Vec::new();
    // Well past the slide, so the cards are at rest.
    push_toasts(&mut scenes, &mut t, content, 8.0, ch, 1_000, None);
    // The top pane's frame, legend and buttons ride the row at its top.
    let title_row_end = content.y + crate::app::gap() + ch;
    assert_eq!(scenes.len(), 2);
    for s in &scenes {
        assert!(s.y >= title_row_end, "card at {} over the title row", s.y);
    }
}

#[test]
fn the_stack_sits_a_cell_inside_the_panes_right_stroke() {
    let mut t = Toasts::default();
    t.push("swarm finished".into(), "done", false, 0);
    let content = Rect {
        x: 0.0,
        y: 40.0,
        w: 800.0,
        h: 600.0,
    };
    let (cw, ch) = (8.0, 16.0);
    let mut scenes = Vec::new();
    push_toasts(&mut scenes, &mut t, content, cw, ch, 1_000, None);
    // The top-right pane's stroke runs a gap in from the content's edge.
    let stroke = content.x + content.w - crate::app::gap();
    let s = &scenes[0];
    assert!(
        (s.x + s.w - (stroke - cw)).abs() < 0.5,
        "right edge {}",
        s.x + s.w
    );
}
