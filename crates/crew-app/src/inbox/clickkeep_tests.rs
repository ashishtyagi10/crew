//! A click or a key acts on what it was aimed at: the row's star beside an
//! open mail, and a key only on the thread it was pressed on, this time.
use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use crate::inbox::wire::Thread;
use crate::inbox::worker::{Got, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

fn pane() -> InboxPane {
    let ((tx, _), (_, rx)) = (mpsc::channel(), mpsc::channel());
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
    p
}

/// Beside an open mail (a pane of 150 columns shows both), a click on
/// another row's ★ stars that row; the open one and the selection stay.
#[test]
fn a_star_clicked_beside_an_open_mail_is_that_rows() {
    let _g = crate::app::motion_test_guard();
    let mut p = pane();
    p.sel = 0;
    p.reading = Some(Reading::new(p.rows[0].clone()));
    let star = p
        .cells(150, 30)
        .into_iter()
        .find(|c| c.c == '\u{2605}')
        .expect("t2's star, in the list beside the mail");
    p.click(150, 30, star.row, star.col);
    assert!(!p.rows[2].starred, "the clicked row's star came off");
    assert!(!p.rows[0].starred, "the open thread is untouched");
    assert_eq!(p.sel, 0, "n still steps from the open thread");
}

/// `a` on a thread, then Esc before it opened: opening it again later does
/// not hand it to smith.
#[test]
fn a_key_left_with_esc_does_not_fire_on_the_next_open() {
    let mut p = pane();
    p.press(&Key::Character("a".into()), 80, 24);
    assert!(p.then.is_some(), "waiting for t0's mails");
    p.press(&Key::Named(NamedKey::Escape), 80, 24);
    assert!(p.reading.is_none(), "left");
    assert!(p.open_at(0), "opened again, by hand");
    p.take(Got::Opened("t0".into(), Vec::new()));
    assert!(p.to_smith.is_none(), "nothing handed over");
}
