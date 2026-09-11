use crate::todopane::item::TodoItem;
use crate::todopane::keys::{apply, TodoInput};
use crate::todopane::test_pane;

const COLS: u16 = 60;
const ROWS: u16 = 20;

fn item(id: u64, who: Option<&str>) -> TodoItem {
    TodoItem {
        id,
        title: format!("t{id}"),
        done: false,
        done_ms: None,
        project: None,
        assignee: who.map(str::to_string),
        due_ms: None,
        due_has_time: false,
        created_ms: id,
        notified: false,
    }
}

#[test]
fn g_on_a_row_bands_the_list_and_g_again_flattens_it() {
    let mut p = test_pane(vec![item(1, Some("sam")), item(2, Some("priya"))]);
    p.sel = Some(0);
    apply(&mut p, TodoInput::Char('g'), COLS, ROWS);
    assert!(p.grouped, "g groups");
    assert_eq!(
        p.order().first().map(|&i| p.items[i].id),
        Some(2),
        "priya sorts before sam, so the list actually re-banded"
    );
    apply(&mut p, TodoInput::Char('g'), COLS, ROWS);
    assert!(!p.grouped, "g again lays it flat");
}

#[test]
fn g_is_inert_in_the_history_and_never_falls_through_to_the_composer() {
    let mut p = test_pane(vec![item(1, Some("sam"))]);
    p.done_view = true;
    p.sel = Some(0);
    apply(&mut p, TodoInput::Char('g'), COLS, ROWS);
    assert!(!p.grouped, "the history bands by day, not by person");
    assert_eq!(p.input, "", "and it must not type a `g` into the composer");
    assert_eq!(p.sel, Some(0), "nor hand focus back");
}
