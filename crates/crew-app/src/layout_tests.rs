use super::*;

fn approx(a: f32, b: f32) {
    assert!((a - b).abs() < 0.5, "{a} != {b}");
}

#[test]
fn one_pane_fills_minus_gap() {
    let r = pane_rects_at(1, 0.0, 0.0, 800.0, 600.0, 0.0);
    assert_eq!(r.len(), 1);
    approx(r[0].x, 0.0);
    approx(r[0].y, 0.0);
    approx(r[0].w, 800.0);
    approx(r[0].h, 600.0);
}

#[test]
fn two_panes_side_by_side() {
    let r = pane_rects_at(2, 0.0, 0.0, 800.0, 600.0, 0.0);
    assert_eq!(r.len(), 2);
    approx(r[0].w, 400.0);
    approx(r[1].x, 400.0);
    approx(r[0].h, 600.0);
}

#[test]
fn four_panes_two_by_two() {
    let r = pane_rects_at(4, 0.0, 0.0, 800.0, 600.0, 0.0);
    assert_eq!(r.len(), 4);
    approx(r[0].w, 400.0);
    approx(r[0].h, 300.0);
    approx(r[3].x, 400.0);
    approx(r[3].y, 300.0);
}

#[test]
fn offset_shifts_origin() {
    let r = pane_rects_at(1, 50.0, 30.0, 800.0, 600.0, 0.0);
    approx(r[0].x, 50.0);
    approx(r[0].y, 30.0);
}

#[test]
fn three_panes_left_column_splits_right_full_height() {
    // Vertical split: two equal columns. The left column carries the
    // surplus pane (split into two rows); the right column stays a single
    // full-height pane.
    let r = pane_rects_at(3, 0.0, 0.0, 800.0, 600.0, 0.0);
    assert_eq!(r.len(), 3);
    // Left column: two stacked half-height tiles.
    approx(r[0].x, 0.0);
    approx(r[0].w, 400.0);
    approx(r[0].h, 300.0);
    approx(r[1].x, 0.0);
    approx(r[1].y, 300.0);
    approx(r[1].h, 300.0);
    // Right column: one full-height tile.
    approx(r[2].x, 400.0);
    approx(r[2].y, 0.0);
    approx(r[2].w, 400.0);
    approx(r[2].h, 600.0);
}

#[test]
fn five_panes_fill_left_columns_first() {
    // n=5 → 3 columns. The surplus (5 - 3 = 2) fills the first two columns
    // (two rows each); the last column stays full height.
    let r = pane_rects_at(5, 0.0, 0.0, 900.0, 600.0, 0.0);
    assert_eq!(r.len(), 5);
    // Columns 0 and 1: two stacked tiles each.
    approx(r[0].x, 0.0);
    approx(r[0].h, 300.0);
    approx(r[1].x, 0.0);
    approx(r[1].y, 300.0);
    approx(r[2].x, 300.0);
    approx(r[3].x, 300.0);
    approx(r[3].y, 300.0);
    // Column 2: a single full-height tile on the right.
    approx(r[4].x, 600.0);
    approx(r[4].y, 0.0);
    approx(r[4].h, 600.0);
}

#[test]
fn full_height_column_keeps_gap_conventions() {
    // With a gap, the full-height right column still keeps full outer
    // margins (right/top/bottom) and a half-gap seam on its inner (left)
    // edge, like every other tile.
    let r = pane_rects_at(3, 0.0, 0.0, 800.0, 600.0, 8.0);
    // r[2] is the right, full-height column.
    approx(r[2].x + r[2].w, 792.0); // full outer margin on the right
    approx(r[2].y, 8.0); // full outer margin on top
    approx(r[2].y + r[2].h, 592.0); // full outer margin on the bottom
                                    // Inner seam against the left column is a single gap.
    approx(r[2].x - (r[0].x + r[0].w), 8.0);
}

#[test]
fn zero_panes_empty() {
    assert!(pane_rects_at(0, 0.0, 0.0, 800.0, 600.0, 4.0).is_empty());
}

#[test]
fn interior_seam_is_one_gap_outer_margin_full() {
    let r = pane_rects_at(2, 0.0, 0.0, 800.0, 600.0, 8.0);
    // Outer margins keep the full gap…
    approx(r[0].x, 8.0);
    approx(r[1].x + r[1].w, 792.0);
    approx(r[0].y, 8.0);
    approx(r[0].h, 584.0);
    // …while the seam between the two panes is a single gap, not two.
    approx(r[1].x - (r[0].x + r[0].w), 8.0);
}
