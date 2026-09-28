use super::*;

fn mark(ch: char, row: u16) -> CellView {
    CellView {
        col: BOX_COL + 1,
        row,
        c: ch,
        fg: (200, 100, 50),
        ..Default::default()
    }
}

/// An open box is an outline, a done one a fill under a drawn tick — and
/// both are square on screen and sit inside the `[ ]` click target.
#[test]
fn the_box_is_square_larger_than_a_cell_and_inside_the_target() {
    let _g = crate::app::theme_test_guard();
    let aspect = 2.1; // cell_h / cell_w
    let (cells, paint) = upgrade(vec![mark(OPEN, 3), mark(DONE, 5)], aspect);
    let outline: Vec<&Paint> = paint.iter().filter(|p| (p.y - 3.0).abs() < 1.0).collect();
    assert_eq!(outline.len(), 4, "four sides: {paint:?}");
    let fill: Vec<&Paint> = paint.iter().filter(|p| (p.y - 5.0).abs() < 1.0).collect();
    assert_eq!(fill.len(), 1, "one fill: {paint:?}");
    let f = fill[0];
    // Square in pixels: w columns of cell_w == h rows of cell_h.
    assert!((f.w - f.h * aspect).abs() < 1e-4, "{f:?}");
    assert!(f.w > 1.0, "wider than the one cell ☐ could use: {f:?}");
    let (lo, hi) = (BOX_COL as f32, (BOX_COL + 3) as f32);
    for p in &paint {
        assert!(
            p.x >= lo && p.x + p.w <= hi,
            "outside the click target: {p:?}"
        );
    }
    assert_eq!(cells[0].c, ' ', "the outline replaces the glyph");
    assert_eq!(cells[1].c, '\u{2714}', "a heavy tick on the fill");
    assert_eq!(
        cells[1].fg,
        crew_theme::theme().page_bg,
        "in the page colour"
    );
}

#[test]
fn cells_that_are_not_the_mark_pass_through() {
    let other = CellView {
        col: BOX_COL + 4,
        row: 1,
        c: OPEN,
        ..Default::default()
    };
    let (cells, paint) = upgrade(vec![other], 2.0);
    assert!(paint.is_empty());
    assert_eq!(cells[0].c, OPEN, "only the mark column is a checkbox");
}
