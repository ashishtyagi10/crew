use super::*;

const CELL: (f32, f32) = (8.0, 17.0);

/// A card 40×10 cells at (100, 200) with a failed command on content row 3.
fn card() -> Rect {
    Rect {
        x: 100.0,
        y: 200.0,
        w: 40.0 * CELL.0,
        h: 10.0 * CELL.1,
    }
}

/// The y of the middle of content row `r` as the card draws it.
fn mid(r: u16) -> f32 {
    let dy = crate::panefit::slack(card(), CELL.0, CELL.1).1 / CELL.1;
    card().y + (crate::cardmarks::row_y(r, dy) + 0.5) * CELL.1
}

#[test]
fn a_click_on_the_failed_rows_border_cell_hits() {
    assert_eq!(
        hit(card(), CELL, (card().x + 2.0, mid(3)), &[3, 5]),
        Some(3)
    );
    assert_eq!(
        hit(card(), CELL, (card().x + 2.0, mid(5)), &[3, 5]),
        Some(5)
    );
}

#[test]
fn the_row_above_and_below_do_not() {
    assert_eq!(None, hit(card(), CELL, (card().x + 2.0, mid(2)), &[3]));
    assert_eq!(None, hit(card(), CELL, (card().x + 2.0, mid(4)), &[3]));
}

#[test]
fn a_border_with_no_failure_hits_nowhere() {
    for r in 0..8 {
        assert_eq!(None, hit(card(), CELL, (card().x + 2.0, mid(r)), &[]));
    }
}

#[test]
fn the_content_and_the_gutter_are_not_the_mark() {
    let content = (card().x + CELL.0 + 1.0, mid(3));
    assert_eq!(None, hit(card(), CELL, content, &[3]), "content");
    assert_eq!(
        None,
        hit(card(), CELL, (card().x - 1.0, mid(3)), &[3]),
        "outside"
    );
}

/// What the card DRAWS for a failed row is what answers the click: the
/// mark's own pixels, not a re-derivation of where they ought to be.
#[test]
fn the_drawn_mark_lies_inside_the_clickable_cell() {
    let _g = crate::app::theme_test_guard();
    let dy = crate::panefit::slack(card(), CELL.0, CELL.1).1 / CELL.1;
    let bar = crate::panecard::Bar {
        fail_rows: &[3],
        ..Default::default()
    };
    let drawn = crate::cardmarks::marks(10, &bar, dy, CELL.0, CELL.1);
    assert_eq!(drawn.len(), 1, "one failed row, one mark");
    let (m, r) = (&drawn[0], card());
    let at = (
        r.x + (m.x + m.w / 2.0) * CELL.0,
        r.y + (m.y + m.h / 2.0) * CELL.1,
    );
    assert_eq!(hit(r, CELL, at, &[3]), Some(3));
}
