use std::sync::mpsc;

use crate::inbox::mouse::Clicked;
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
    assert!(matches!(p.click(80, 24, r, 20), Clicked::Done));
    assert_eq!(
        (p.sel, p.reading.is_none()),
        (1, true),
        "selected, not opened"
    );
    assert!(p.motion.glide.is_some(), "its wash glides there");
    assert!(matches!(p.click(80, 24, r, 20), Clicked::Done));
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
    assert!(matches!(p.click(80, 24, star.row, star.col), Clicked::Done));
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
    assert!(matches!(p.click(150, 24, r, 10), Clicked::Done));
    assert_eq!(p.reading.as_ref().map(|r| r.thread.id.as_str()), Some("t3"));
    assert_eq!(
        p.thread_at(100, 24, r, 10),
        None,
        "narrow: the reader has the pane"
    );
}

#[test]
fn a_keycap_in_the_foot_is_its_key_and_the_counters_arrows_step_through_the_list() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = pane();
    p.sel = 1;
    p.reading = Some(Reading::new(p.rows[1].clone()));
    let cells = p.cells(80, 24);
    let cap = cells
        .iter()
        .find(|c| c.c == 'n' && c.mark && c.row > 18)
        .expect("n's keycap");
    assert!(matches!(p.click(80, 24, cap.row, cap.col), Clicked::Done));
    assert_eq!(
        p.reading.as_ref().unwrap().thread.id,
        "t2",
        "n: the next thread"
    );
    let row: String = {
        let mut on: Vec<_> = p.cells(80, 24).into_iter().filter(|c| c.row == 1).collect();
        on.sort_by_key(|c| c.col);
        on.iter().map(|c| c.c).collect()
    };
    assert!(row.contains("3 of 4"), "{row}");
    let (s, start) = p.counter(80).unwrap();
    assert!(matches!(p.click(80, 24, 1, start), Clicked::Done), "‹");
    assert_eq!(p.sel, 1);
    let end = start + crate::chatwidth::str_w(&s) as u16 - 1;
    assert!(matches!(p.click(80, 24, 1, end), Clicked::Done), "›");
    assert_eq!(p.sel, 2);
    let esc = cells
        .iter()
        .find(|c| c.c == 'e' && c.mark)
        .expect("esc's keycap");
    assert!(matches!(p.click(80, 24, esc.row, esc.col), Clicked::Done));
    assert!(p.reading.is_none(), "esc: back to the list");
}
