//! The delete affordance is the `✗` and the air after it — not the scroll
//! gutter beside them. The gutter is drawn only while the list overflows,
//! which is exactly when a reader reaches for it, so a delete zone that
//! reached the pane's last column made the thumb a trap.
use super::{del_col, del_zone};
use crate::todopane::item::TodoItem;
use crate::todopane::render::{click_at, TodoClick};
use crate::todopane::test_pane;

const COLS: u16 = 40;
const ROWS: u16 = 6;

fn item(id: u64, title: &str) -> TodoItem {
    TodoItem {
        id,
        title: title.to_string(),
        done: false,
        done_ms: None,
        project: None,
        due_ms: None,
        due_has_time: false,
        created_ms: id,
        notified: false,
    }
}

#[test]
fn the_zone_is_the_glyph_and_one_cell_of_air() {
    assert_eq!(del_zone(COLS), del_col(COLS)..del_col(COLS) + 2);
    assert!(
        !del_zone(COLS).contains(&(COLS - 1)),
        "the gutter column is not a delete"
    );
}

#[test]
fn clicking_the_gutter_of_an_overflowing_list_does_not_delete() {
    // Ten items in six rows: the list overflows, so the thumb is drawn at
    // the last content column on every list row.
    let items = (1..=10).map(|i| item(i, &format!("item {i}"))).collect();
    let p = test_pane(items);
    let cells = crate::todopane::render::cells(&p, COLS, ROWS);
    assert!(
        cells.iter().any(|c| c.col == COLS - 1 && c.c == '\u{2503}'),
        "the thumb is on the last column"
    );
    assert_eq!(
        click_at(&p, 0, COLS - 1, COLS, ROWS),
        Some(TodoClick::Select(0))
    );
    assert_eq!(
        click_at(&p, 0, COLS - 3, COLS, ROWS),
        Some(TodoClick::Delete(0))
    );
    assert_eq!(
        click_at(&p, 0, COLS - 2, COLS, ROWS),
        Some(TodoClick::Delete(0))
    );
}
