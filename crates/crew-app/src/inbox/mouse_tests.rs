use std::sync::mpsc;

use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

fn pane() -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = (0..4)
        .map(|i| Thread {
            id: format!("t{i}"),
            from: format!("P{i} <p{i}@x.org>"),
            subject: format!("subject {i}"),
            starred: i == 2,
            count: 1,
            ..Thread::default()
        })
        .collect();
    (p, asks)
}

/// The first row thread `i` is drawn on, in a pane `cols` by 24.
fn row_of(p: &InboxPane, cols: u16, i: usize) -> u16 {
    let shown = p.shown(p.body_rows(cols, 24));
    let at = shown.threads().find(|&(_, j)| j == i).unwrap().0;
    at
}

#[test]
fn a_click_selects_a_second_opens_and_the_star_comes_off_where_it_is() {
    let _g = crate::app::motion_test_guard();
    let (mut p, asks) = pane();
    let r = row_of(&p, 80, 1);
    assert_eq!(p.thread_at(80, 24, r, 20), Some(1));
    assert_eq!(p.thread_at(80, 24, 0, 20), None, "the air above the list");
    assert!(p.click(80, 24, r, 20));
    assert_eq!(
        (p.sel, p.reading.is_none()),
        (1, true),
        "selected, not opened"
    );
    assert!(p.motion.glide.is_some(), "its wash glides there");
    assert!(p.click(80, 24, r, 20));
    assert_eq!(p.reading.as_ref().map(|r| r.thread.id.as_str()), Some("t1"));
    assert!(asks
        .try_iter()
        .any(|a| matches!(a, Ask::Open(id) if id == "t1")));
    p.reading = None;
    let r2 = row_of(&p, 80, 2);
    let cells = p.cells(80, 24);
    let star = cells
        .iter()
        .find(|c| c.c == '\u{2605}' && c.row == r2)
        .unwrap();
    assert!(p.click(80, 24, star.row, star.col));
    assert!(!p.rows[2].starred, "the star came off");
    assert_eq!(p.sel, 2);
}

#[test]
fn beside_an_open_thread_a_click_on_the_list_opens_at_once() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = pane();
    p.reading = Some(Reading::new(p.rows[0].clone()));
    let r = row_of(&p, 150, 3);
    assert_eq!(p.thread_at(150, 24, r, 70), None, "the reader's half");
    assert!(p.click(150, 24, r, 10));
    assert_eq!(p.reading.as_ref().map(|r| r.thread.id.as_str()), Some("t3"));
    assert_eq!(
        p.thread_at(100, 24, r, 10),
        None,
        "narrow: the reader has the pane"
    );
}
