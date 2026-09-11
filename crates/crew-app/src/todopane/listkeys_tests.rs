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
fn a_named_list_bands_with_no_key_pressed_and_g_flattens_it() {
    let mut p = test_pane(vec![item(1, Some("sam")), item(2, Some("priya"))]);
    p.sel = Some(0);
    assert!(p.grouped, "a list with people on it bands by default");
    assert_eq!(
        p.order().first().map(|&i| p.items[i].id),
        Some(2),
        "priya sorts before sam, so the untouched list is already banded"
    );
    apply(&mut p, TodoInput::Char('g'), COLS, ROWS);
    assert!(!p.grouped, "g lays it flat");
    assert_eq!(
        p.order().first().map(|&i| p.items[i].id),
        Some(1),
        "flat is creation order again, not priya first"
    );
    apply(&mut p, TodoInput::Char('g'), COLS, ROWS);
    assert!(p.grouped, "g again bands it back");
}

#[test]
fn a_list_with_nobody_named_draws_no_bands_even_though_grouping_is_on() {
    let p = test_pane(vec![item(1, None), item(2, None)]);
    assert!(p.grouped, "the flag is on");
    assert_eq!(
        p.bands(),
        crate::todopane::Bands::None,
        "but one `unassigned` header over the whole list says nothing"
    );
}

#[test]
fn g_is_inert_in_the_history_and_never_falls_through_to_the_composer() {
    let mut p = test_pane(vec![item(1, Some("sam"))]);
    p.done_view = true;
    p.sel = Some(0);
    apply(&mut p, TodoInput::Char('g'), COLS, ROWS);
    assert!(p.grouped, "g is inert in here — it did not flip the flag");
    assert_eq!(
        p.bands(),
        crate::todopane::Bands::Days,
        "the history bands by day, not by person"
    );
    assert_eq!(p.input, "", "and it must not type a `g` into the composer");
    assert_eq!(p.sel, Some(0), "nor hand focus back");
}
