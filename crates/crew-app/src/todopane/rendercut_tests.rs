//! A title the pane cannot finish says it was cut.
use super::*;
use crate::todopane::item::TodoItem;
use crate::todopane::test_pane;

fn text(cells: &[crew_render::CellView]) -> String {
    let mut v: Vec<_> = cells.iter().collect();
    v.sort_by_key(|c| (c.row, c.col));
    v.iter().map(|c| c.c).collect()
}

#[test]
fn a_title_cut_by_the_pane_ends_in_an_ellipsis() {
    let long = "work out why the atlas grows on the first frame after a theme switch";
    let item = TodoItem {
        id: 1,
        title: long.into(),
        created_ms: 1,
        ..Default::default()
    };
    let p = test_pane(vec![item]);
    // Room for the title's first row and the composer, not its second row.
    let short = text(&cells(&p, 40, 3));
    assert!(short.contains('\u{2026}'), "{short:?}");
    assert!(!short.contains("switch"), "{short:?}");
    // Tall enough for every row: whole, no mark.
    let tall = text(&cells(&p, 40, 20));
    assert!(
        tall.contains("switch") && !tall.contains('\u{2026}'),
        "{tall:?}"
    );
}
