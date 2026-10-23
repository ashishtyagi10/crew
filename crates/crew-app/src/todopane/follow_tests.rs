//! A write from elsewhere (the Reminders sync, a second pane, `t` in the
//! inbox) keeps the highlight on the todo it was on. Before, it kept the row
//! NUMBER: a reminder ticked or added on the phone shifted the rows under
//! it, and `d` — no undo — deleted the todo that slid into place, here and,
//! on the next pass, on the phone too.
use super::super::item::TodoItem;
use super::super::store;
use super::super::TodoPane;

fn item(id: u64, due: Option<u64>) -> TodoItem {
    TodoItem {
        id,
        title: format!("t{id}"),
        done: false,
        done_ms: None,
        project: None,
        assignee: None,
        due_ms: due,
        due_has_time: false,
        created_ms: id,
        notified: false,
        run: None,
    }
}

const DAY: u64 = 86_400_000;
const MON: u64 = 2_000_000_000_000;

#[test]
fn a_todo_added_above_moves_the_rows_and_the_highlight_goes_with_its_todo() {
    let _g = store::test_guard(vec![item(1, Some(MON)), item(2, None)]);
    let mut p = TodoPane::new();
    p.sel = Some(1);
    assert_eq!(p.id_at(1), Some(2));
    store::mutate(|items| items.push(item(3, Some(MON - DAY))));
    assert!(p.poll());
    assert_eq!(p.sel.and_then(|s| p.id_at(s)), Some(2), "still on t2");
}

#[test]
fn a_todo_gone_from_above_keeps_the_highlight_on_its_todo() {
    let _g = store::test_guard(vec![
        item(1, Some(MON)),
        item(2, Some(MON + DAY)),
        item(3, None),
        item(4, None),
    ]);
    let mut p = TodoPane::new();
    p.sel = Some(2);
    store::mutate(|items| items[0].done = true);
    p.poll();
    assert_eq!(p.sel.and_then(|s| p.id_at(s)), Some(3), "still on t3");
}

#[test]
fn when_its_own_todo_goes_the_highlight_stays_in_place_and_in_range() {
    let _g = store::test_guard(vec![item(1, Some(MON)), item(2, None)]);
    let mut p = TodoPane::new();
    p.sel = Some(1);
    store::mutate(|items| items.retain(|it| it.id != 2));
    p.poll();
    assert_eq!(p.sel, Some(0));
}
