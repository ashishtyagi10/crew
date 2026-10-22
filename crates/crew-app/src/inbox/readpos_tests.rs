use std::sync::mpsc;

use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Got, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

fn row(id: &str) -> Thread {
    Thread {
        id: id.into(),
        subject: format!("about {id}"),
        ..Thread::default()
    }
}

#[test]
fn a_thread_filed_elsewhere_leaves_no_false_count_and_n_opens_what_took_its_place() {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = vec![row("a"), row("b"), row("c")];
    p.sel = 1;
    p.reading = Some(Reading::new(row("b")));
    assert_eq!(p.counter().as_deref(), Some("\u{2039}  2 of 3  \u{203a}"));
    // Archived in Gmail on the web: the next check's page has no `b`.
    p.take(Got::Page {
        query: String::new(),
        rows: vec![row("a"), row("c")],
        next: None,
        more: false,
        whole: true,
    });
    assert_eq!(p.sel, 1, "the place it had, now `c`'s, not the top");
    assert_eq!(p.counter(), None, "no place of its own to count");
    assert_eq!(p.next_thread(true), "");
    assert_eq!(
        p.reading.as_ref().unwrap().thread.id,
        "c",
        "n: the one in its place"
    );
    assert!(asks
        .try_iter()
        .any(|a| matches!(a, Ask::Open(id) if id == "c")));
    assert_eq!(p.counter().as_deref(), Some("\u{2039}  2 of 2  \u{203a}"));
}
