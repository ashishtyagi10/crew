use super::*;

fn item(id: u64, due: Option<u64>, done: bool, project: Option<&str>, created: u64) -> TodoItem {
    TodoItem {
        id,
        title: format!("t{id}"),
        done,
        done_ms: None,
        project: project.map(str::to_string),
        due_ms: due,
        due_has_time: false,
        created_ms: created,
        notified: false,
    }
}

const NOW: u64 = 1_000_000;

#[test]
fn overdue_then_due_then_undated_and_done_hidden() {
    let items = vec![
        item(1, None, false, None, 10),            // undated
        item(2, Some(NOW + 500), false, None, 20), // upcoming
        item(3, Some(NOW - 500), false, None, 30), // overdue
        item(4, Some(NOW - 900), true, None, 40),  // done (even though past)
        item(5, Some(NOW - 100), false, None, 50), // overdue, later than 3
        item(6, Some(NOW + 100), false, None, 60), // upcoming, sooner than 2
    ];
    let order = display_order(&items, None, false);
    let ids: Vec<u64> = order.iter().map(|&i| items[i].id).collect();
    assert_eq!(ids, vec![3, 5, 6, 2, 1]);
}

#[test]
fn undated_items_keep_creation_order() {
    let items = vec![
        item(1, None, false, None, 300),
        item(2, None, false, None, 100),
        item(3, None, false, None, 200),
    ];
    let order = display_order(&items, None, false);
    let ids: Vec<u64> = order.iter().map(|&i| items[i].id).collect();
    assert_eq!(ids, vec![2, 3, 1]);
}

#[test]
fn done_items_are_hidden_regardless_of_due_or_filter() {
    let items = vec![
        item(1, Some(NOW - 999), true, Some("crew"), 10),
        item(2, None, false, Some("crew"), 20),
    ];
    let ids = |filter| -> Vec<u64> {
        display_order(&items, filter, false)
            .iter()
            .map(|&i| items[i].id)
            .collect()
    };
    assert_eq!(ids(None), vec![2], "a done overdue item is hidden");
    assert_eq!(ids(Some("crew")), vec![2], "hidden under a filter too");
}

#[test]
fn the_project_filter_is_case_insensitive_and_drops_untagged() {
    let items = vec![
        item(1, None, false, Some("Crew"), 10),
        item(2, None, false, Some("home"), 20),
        item(3, None, false, None, 30),
        item(4, None, false, Some("crew"), 40),
    ];
    let order = display_order(&items, Some("crew"), false);
    let ids: Vec<u64> = order.iter().map(|&i| items[i].id).collect();
    assert_eq!(ids, vec![1, 4]);
}

/// The way back for done items: hidden by default, `show_done` sinks them
/// below every open item — newest completion first — and the filter still
/// applies to them.
#[test]
fn show_done_sinks_done_items_newest_completion_first() {
    let items = vec![
        item(1, None, false, None, 10),        // open
        item(2, None, true, None, 20),         // done early
        item(3, None, true, None, 30),         // done late
        item(4, None, true, Some("home"), 40), // done, tagged
    ];
    assert_eq!(
        display_order(&items, None, false),
        vec![0],
        "hidden by default"
    );
    assert_eq!(
        display_order(&items, None, true),
        vec![0, 3, 2, 1],
        "open first, then done newest-first"
    );
    assert_eq!(
        display_order(&items, Some("home"), true),
        vec![3],
        "the filter still governs done rows"
    );
}

// --- done history (v0.17: `/todo done`) -----------------------------------

fn done_at(id: u64, stamp: Option<u64>, created: u64) -> TodoItem {
    let mut it = item(id, None, true, None, created);
    it.done_ms = stamp;
    it
}

#[test]
fn done_order_is_newest_stamp_first_then_legacy_by_creation() {
    let items = vec![
        item(1, None, false, None, 10), // open — not history
        done_at(2, Some(500), 40),      // stamped, older tick
        done_at(3, Some(900), 20),      // stamped, newest tick
        done_at(4, None, 999),          // legacy tick (pre-stamp), newer created
        done_at(5, None, 100),          // legacy tick, older created
    ];
    let ids: Vec<u64> = done_order(&items, None)
        .iter()
        .map(|&i| items[i].id)
        .collect();
    assert_eq!(
        ids,
        vec![3, 2, 4, 5],
        "stamped ticks newest-first, every legacy tick after them"
    );
}

#[test]
fn done_order_honours_the_project_filter() {
    let mut a = done_at(1, Some(100), 1);
    a.project = Some("crew".into());
    let b = done_at(2, Some(200), 2);
    let items = vec![a, b];
    let ids: Vec<u64> = done_order(&items, Some("CREW"))
        .iter()
        .map(|&i| items[i].id)
        .collect();
    assert_eq!(ids, vec![1], "case-insensitive; untagged rows drop out");
}

/// `show_done`'s sunk rows must sort by the honest completion stamp when
/// one exists — not the `created_ms` proxy the pre-stamp build used.
#[test]
fn show_done_rank_prefers_the_completion_stamp_over_creation() {
    let items = vec![
        done_at(1, Some(900), 10), // created first, ticked LAST
        done_at(2, Some(500), 999),
        item(3, None, false, None, 5), // open stays above the sunk rows
    ];
    let ids: Vec<u64> = display_order(&items, None, true)
        .iter()
        .map(|&i| items[i].id)
        .collect();
    assert_eq!(ids, vec![3, 1, 2]);
}
