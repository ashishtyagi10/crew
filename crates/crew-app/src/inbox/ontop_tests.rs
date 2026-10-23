use std::sync::mpsc;

use super::order;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Stage};

fn th(id: &str, at_ms: i64, unread: bool) -> Thread {
    Thread {
        id: id.into(),
        at_ms,
        unread,
        ..Thread::default()
    }
}

fn ids(rows: &[Thread]) -> Vec<&str> {
    rows.iter().map(|t| t.id.as_str()).collect()
}

fn pane() -> InboxPane {
    let (tx, _) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    InboxPane::with(Stage::Ready(Worker { tx, rx }))
}

#[test]
fn unread_threads_go_on_top_each_part_newest_first() {
    let mut rows = vec![
        th("a", 50, false),
        th("b", 40, true),
        th("c", 30, false),
        th("d", 20, true),
    ];
    order(&mut rows, &[]);
    assert_eq!(ids(&rows), ["b", "d", "a", "c"]);
    let on: Vec<bool> = rows.iter().map(|t| t.on_top).collect();
    assert_eq!(on, [true, true, false, false]);
    let mut again = rows.clone();
    again[0].unread = false;
    order(&mut again, &["b".into(), "d".into()]);
    assert_eq!(ids(&again), ["b", "d", "a", "c"], "b was on top: it stays");
    assert!(again[0].on_top);
}

#[test]
fn a_thread_read_here_keeps_its_place_until_the_list_is_asked_for_again() {
    let mut p = pane();
    let first = |b| vec![th("a", 50, false), th("b", 40, b), th("c", 30, true)];
    p.paged(first(true), true, false, false);
    assert_eq!(ids(&p.rows), ["b", "c", "a"]);
    assert_eq!(p.sel, 0, "the first list opens on the newest unread");
    assert!(p.open_at(0));
    assert!(!p.rows[0].unread, "opened is read");
    // The check after the read lists the first page again, b read in it.
    p.paged(first(false), true, false, false);
    assert_eq!(ids(&p.rows), ["b", "c", "a"], "b stays where it was read");
    assert_eq!(p.sel, 0);
    p.search(""); // R: the list again, from the top
    p.paged(first(false), true, false, false);
    assert_eq!(ids(&p.rows), ["c", "a", "b"]);
}

#[test]
fn unread_on_a_further_page_goes_on_top_and_the_selection_keeps_its_thread() {
    let mut p = pane();
    let first = vec![th("a", 50, true), th("b", 40, false), th("c", 30, false)];
    p.paged(first, true, false, false);
    p.sel = 2; // c, the last row, where j asks for more
    p.paged(
        vec![th("d", 20, true), th("e", 10, false)],
        false,
        true,
        false,
    );
    assert_eq!(ids(&p.rows), ["a", "d", "b", "c", "e"]);
    assert_eq!(p.rows[p.sel].id, "c");
    // …and says so: `j` and `n` go on down, and would never meet `d`.
    assert_eq!(p.note, "1 more unread on top");
}

#[test]
fn the_threads_on_top_have_a_band_of_their_own_and_the_days_go_on_under_them() {
    let mut p = pane();
    let now = chrono::Utc::now().timestamp_millis();
    p.paged(
        vec![th("a", now, false), th("b", now - 1, true)],
        true,
        false,
        false,
    );
    let bands: Vec<String> = (p.shown(30).slots.into_iter())
        .filter_map(|(_, s)| match s {
            crate::inbox::layout::Slot::Band(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(bands, ["unread", "today"]);
}
