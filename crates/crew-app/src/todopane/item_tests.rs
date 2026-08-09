use super::*;

fn item(id: u64, due: Option<u64>, done: bool, project: Option<&str>, created: u64) -> TodoItem {
    TodoItem {
        id,
        title: format!("t{id}"),
        done,
        project: project.map(str::to_string),
        due_ms: due,
        due_has_time: false,
        created_ms: created,
        notified: false,
    }
}

const NOW: u64 = 1_000_000;

#[test]
fn overdue_then_due_then_undated_then_done() {
    let items = vec![
        item(1, None, false, None, 10),            // undated
        item(2, Some(NOW + 500), false, None, 20), // upcoming
        item(3, Some(NOW - 500), false, None, 30), // overdue
        item(4, Some(NOW - 900), true, None, 40),  // done (even though past)
        item(5, Some(NOW - 100), false, None, 50), // overdue, later than 3
        item(6, Some(NOW + 100), false, None, 60), // upcoming, sooner than 2
    ];
    let order = display_order(&items, None);
    let ids: Vec<u64> = order.iter().map(|&i| items[i].id).collect();
    assert_eq!(ids, vec![3, 5, 6, 2, 1, 4]);
}

#[test]
fn undated_items_keep_creation_order() {
    let items = vec![
        item(1, None, false, None, 300),
        item(2, None, false, None, 100),
        item(3, None, false, None, 200),
    ];
    let order = display_order(&items, None);
    let ids: Vec<u64> = order.iter().map(|&i| items[i].id).collect();
    assert_eq!(ids, vec![2, 3, 1]);
}

#[test]
fn done_items_sink_regardless_of_due() {
    let items = vec![
        item(1, Some(NOW - 999), true, None, 10),
        item(2, None, false, None, 20),
    ];
    let order = display_order(&items, None);
    let ids: Vec<u64> = order.iter().map(|&i| items[i].id).collect();
    assert_eq!(ids, vec![2, 1], "a done overdue item still sinks");
}

#[test]
fn the_project_filter_is_case_insensitive_and_drops_untagged() {
    let items = vec![
        item(1, None, false, Some("Crew"), 10),
        item(2, None, false, Some("home"), 20),
        item(3, None, false, None, 30),
        item(4, None, false, Some("crew"), 40),
    ];
    let order = display_order(&items, Some("crew"));
    let ids: Vec<u64> = order.iter().map(|&i| items[i].id).collect();
    assert_eq!(ids, vec![1, 4]);
}
