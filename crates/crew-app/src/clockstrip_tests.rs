//! The nav clock's strip is centred by display width and keeps a column of
//! air at the edge, like every other nav row.
use crew_render::CellView;

fn row(cells: &[CellView], r: u16) -> Vec<&CellView> {
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
    v.sort_by_key(|c| c.col);
    v
}

#[test]
fn a_wide_glyph_strip_is_placed_by_width_and_inside_the_edge() {
    let _g = crate::app::theme_test_guard();
    let cells = super::clock_cells(
        "12:00",
        "wed sep 10",
        Some("\u{2600} 24\u{b0} \u{65e5}\u{672c}"),
        20,
    );
    let strip = row(&cells, 3);
    assert!(!strip.is_empty());
    assert!(strip.iter().all(|c| c.col < 19), "one column of air");
    // The CJK glyphs advance two columns: no two cells share a column and
    // the one after a wide glyph is two away.
    let cols: Vec<u16> = strip.iter().map(|c| c.col).collect();
    assert!(cols.windows(2).all(|w| w[1] > w[0]), "{cols:?}");
    let jp = strip.iter().position(|c| c.c == '\u{65e5}').unwrap();
    assert_eq!(cols[jp + 1], cols[jp] + 2, "{cols:?}");
}
