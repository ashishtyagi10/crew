use super::*;
use crate::todopane::item::TodoItem;
use crate::todopane::test_pane;

fn item(id: u64, who: Option<&str>, due: Option<u64>) -> TodoItem {
    TodoItem {
        id,
        title: format!("t{id}"),
        done: false,
        done_ms: None,
        project: None,
        assignee: who.map(str::to_string),
        due_ms: due,
        due_has_time: false,
        created_ms: id,
        notified: false,
    }
}

const HOUR: u64 = 3_600_000;

#[test]
fn people_are_alphabetical_and_case_folded() {
    let items = vec![
        item(1, Some("Sam"), None),
        item(2, Some("priya"), None),
        item(3, Some("PRIYA"), None),
        item(4, None, None),
    ];
    assert_eq!(
        people(&items, Filters::default()),
        vec!["priya".to_string(), "Sam".to_string()],
        "first-seen spelling, deduped case-insensitively, unassigned absent"
    );
}

#[test]
fn bands_keep_the_flat_ordering_inside_each_person() {
    // priya's two items are due out of creation order; grouping must not
    // re-sort them, only gather them.
    let items = vec![
        item(1, Some("sam"), Some(10 * HOUR)),
        item(2, Some("priya"), Some(30 * HOUR)),
        item(3, Some("priya"), Some(20 * HOUR)),
        item(4, None, Some(5 * HOUR)),
    ];
    let ids: Vec<u64> = order(&items, Filters::default(), false)
        .into_iter()
        .map(|i| items[i].id)
        .collect();
    assert_eq!(
        ids,
        vec![3, 2, 1, 4],
        "priya (due asc), then sam, then unassigned last"
    );
}

#[test]
fn a_band_opens_on_the_first_row_of_each_person() {
    let items = vec![
        item(1, Some("priya"), None),
        item(2, Some("priya"), None),
        item(3, Some("sam"), None),
        item(4, None, None),
    ];
    let mut p = test_pane(items);
    p.grouped = true;
    let ord = p.order();
    let heads: Vec<bool> = (0..ord.len()).map(|di| starts(&p, &ord, di)).collect();
    assert_eq!(heads, vec![true, false, true, true]);
    p.grouped = false;
    let ord = p.order();
    assert!(
        (0..ord.len()).all(|di| !starts(&p, &ord, di)),
        "a flat list has no bands at all"
    );
}

#[test]
fn the_tally_counts_open_overdue_and_todays_ticks() {
    let now = 100 * HOUR;
    let mut items = vec![
        item(1, Some("priya"), Some(now - HOUR)), // overdue
        item(2, Some("priya"), Some(now + HOUR)), // open, not due
        item(3, Some("priya"), None),             // open, undated
        item(4, Some("sam"), None),               // someone else's
    ];
    let mut ticked = item(5, Some("priya"), None);
    ticked.done = true;
    ticked.done_ms = Some(now - HOUR);
    let mut old = item(6, Some("priya"), None);
    old.done = true;
    old.done_ms = Some(now - 40 * HOUR);
    items.push(ticked);
    items.push(old);

    assert_eq!(
        tally(&items, Filters::default(), Some("priya"), now),
        "3 open \u{b7} 1 overdue \u{b7} 1 done today",
        "yesterday's tick is history, not today's standup"
    );
    assert_eq!(
        tally(&items, Filters::default(), None, now),
        "0 open \u{b7} 0 overdue \u{b7} 0 done today",
        "nothing is unassigned here"
    );
}

#[test]
fn the_tally_stays_inside_an_active_project_filter() {
    let now = 100 * HOUR;
    let mut items = vec![item(1, Some("priya"), None), item(2, Some("priya"), None)];
    items[0].project = Some("crew".to_string());
    let f = Filters {
        project: Some("crew"),
        who: None,
    };
    assert_eq!(
        tally(&items, f, Some("priya"), now),
        "1 open \u{b7} 0 overdue \u{b7} 0 done today",
        "the other project's item is not on this board"
    );
}
