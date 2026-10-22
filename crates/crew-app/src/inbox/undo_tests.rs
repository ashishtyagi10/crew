use std::sync::mpsc;

use winit::keyboard::Key;

use super::super::wire::Thread;
use super::super::worker::{Ask, Got, Worker};
use super::super::{InboxPane, Stage};

/// A signed-in pane over threads `ids`, and what it asks.
fn pane(ids: &[&str]) -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = ids
        .iter()
        .map(|id| Thread {
            id: id.to_string(),
            subject: format!("about {id}"),
            ..Thread::default()
        })
        .collect();
    (p, asks)
}

fn relabel(asks: &mpsc::Receiver<Ask>) {
    assert!(
        matches!(asks.try_recv(), Ok(Ask::Relabel { .. })),
        "no relabel sent"
    );
}

fn ch(c: &str) -> Key {
    Key::Character(c.into())
}

/// A refused archive puts the row back where it was and says why, where the
/// row came back unsaid (the list's page cleared the words), and `z` has
/// nothing left to take back: it added the row a second time.
#[test]
fn a_refused_archive_is_said_and_put_back_once() {
    let (mut p, asks) = pane(&["a", "b", "c"]);
    p.sel = 1;
    p.press(&ch("e"), 80, 24);
    relabel(&asks);
    p.take(Got::Refused("b".into(), "Gmail 403: Forbidden".into()));
    let ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        (ids, p.sel),
        (vec!["a", "b", "c"], 2),
        "back, the selection on c"
    );
    assert!(
        p.note.contains("refused") && p.note.contains("403"),
        "{}",
        p.note
    );
    p.press(&ch("z"), 80, 24);
    assert_eq!(p.note, "nothing to undo");
    assert_eq!(p.rows.len(), 3);
}

/// `z` after a list that came again with the row in it selects the row and
/// does not add it twice.
#[test]
fn z_never_lists_a_thread_twice() {
    let (mut p, asks) = pane(&["a", "b", "c"]);
    p.sel = 1;
    p.press(&ch("e"), 80, 24);
    relabel(&asks);
    let b = Thread {
        id: "b".into(),
        ..Thread::default()
    };
    p.rows.insert(1, b);
    p.press(&ch("z"), 80, 24);
    let ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    assert_eq!((ids, p.sel), (vec!["a", "b", "c"], 1));
}
