use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use super::super::wire::Thread;
use super::super::worker::{Ask, Got, Worker};
use super::super::{InboxPane, Stage};

fn pane(ids: &[&str]) -> (InboxPane, mpsc::Receiver<Ask>, mpsc::Sender<Got>) {
    let (tx, asks) = mpsc::channel();
    let (answer, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = ids
        .iter()
        .map(|id| Thread {
            id: id.to_string(),
            subject: format!("about {id}"),
            unread: true,
            ..Thread::default()
        })
        .collect();
    (p, asks, answer)
}

fn relabel(asks: &mpsc::Receiver<Ask>) -> (String, Vec<&'static str>, Vec<&'static str>) {
    match asks.try_recv() {
        Ok(Ask::Relabel { id, add, remove }) => (id, add, remove),
        _ => panic!("no relabel sent"),
    }
}

fn ch(c: &str) -> Key {
    Key::Character(c.into())
}

#[test]
fn e_archives_by_taking_inbox_away_and_z_puts_it_back_where_it_was() {
    let (mut p, asks, _a) = pane(&["a", "b", "c"]);
    p.sel = 1;
    p.press(&ch("e"), 80, 24);
    assert_eq!(relabel(&asks), ("b".into(), vec![], vec!["INBOX"]));
    let ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["a", "c"]);
    assert!(
        p.note.starts_with("archived \u{201c}about b\u{201d}"),
        "{}",
        p.note
    );
    p.press(&ch("z"), 80, 24);
    assert_eq!(relabel(&asks), ("b".into(), vec!["INBOX"], vec![]));
    let ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    assert_eq!((ids, p.sel), (vec!["a", "b", "c"], 1));
    p.press(&ch("z"), 80, 24);
    assert_eq!(p.note, "nothing to undo");
}

#[test]
fn d_moves_the_thread_to_trash_and_z_takes_it_out_again() {
    let (mut p, asks, _a) = pane(&["a", "b", "c"]);
    p.sel = 2;
    p.press(&ch("d"), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::Trash { id, back: false }) if id == "c"));
    let ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    assert_eq!((ids, p.sel), (vec!["a", "b"], 1));
    assert!(
        p.note.starts_with("deleted \u{201c}about c\u{201d}"),
        "{}",
        p.note
    );
    p.press(&ch("z"), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::Trash { id, back: true }) if id == "c"));
    let ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    assert_eq!((ids, p.sel), (vec!["a", "b", "c"], 2));
}

#[test]
fn shift_r_re_lists_and_r_opens_the_thread_for_a_reply() {
    let (mut p, asks, _a) = pane(&["a"]);
    p.press(&ch("R"), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::List(q)) if q.is_empty()));
    let (mut p, asks, _a) = pane(&["a"]);
    p.press(&ch("r"), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::Open(id)) if id == "a"));
    assert!(
        p.reading.is_some(),
        "the reply waits for the thread to land"
    );
}

#[test]
fn m_and_s_toggle_and_undo_restores_the_flag() {
    let (mut p, asks, _a) = pane(&["a"]);
    p.press(&ch("m"), 80, 24);
    assert_eq!(relabel(&asks), ("a".into(), vec![], vec!["UNREAD"]));
    assert!(!p.rows[0].unread);
    p.press(&ch("s"), 80, 24);
    assert_eq!(relabel(&asks), ("a".into(), vec!["STARRED"], vec![]));
    assert!(p.rows[0].starred);
    p.press(&ch("z"), 80, 24);
    assert_eq!(relabel(&asks), ("a".into(), vec![], vec!["STARRED"]));
    assert!(
        !p.rows[0].starred && !p.rows[0].unread,
        "only the star came back off"
    );
}

#[test]
fn opening_a_thread_marks_it_read_and_archiving_it_reads_on() {
    let (mut p, asks, _a) = pane(&["a", "b"]);
    p.press(&Key::Named(NamedKey::Enter), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::Open(id)) if id == "a"));
    assert_eq!(relabel(&asks), ("a".into(), vec![], vec!["UNREAD"]));
    assert!(p.reading.as_ref().is_some_and(|r| r.thread.id == "a"));
    p.press(&ch("e"), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::Open(id)) if id == "b"));
    assert_eq!(relabel(&asks), ("b".into(), vec![], vec!["UNREAD"]));
    assert_eq!(relabel(&asks), ("a".into(), vec![], vec!["INBOX"]));
    assert!(
        p.reading.as_ref().is_some_and(|r| r.thread.id == "b"),
        "an archived thread gives way to the next"
    );
    assert_eq!(p.rows.len(), 1);
}

#[test]
fn a_read_only_grant_is_told_how_to_file() {
    let (mut p, _asks, answer) = pane(&["a"]);
    answer
        .send(Got::Failed(
            "Gmail 403: Request had insufficient authentication scopes.".into(),
        ))
        .unwrap();
    p.poll();
    assert!(p.note.contains("/inbox signout"), "{}", p.note);
}
