use std::sync::mpsc;

use winit::keyboard::Key;

use crate::inbox::api::Mail;
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Got, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

/// A pane listing `ids`, every one unread, reading `open`.
fn reading(ids: &[&str], open: usize) -> (InboxPane, mpsc::Receiver<Ask>, mpsc::Sender<Got>) {
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
    p.sel = open;
    p.reading = Some(Reading::new(p.rows[open].clone()));
    (p, asks, answer)
}

fn press(p: &mut InboxPane, k: &str) {
    p.press(&Key::Character(k.into()), 80, 24);
}

fn open_id(p: &InboxPane) -> Option<&str> {
    p.reading.as_ref().map(|r| r.thread.id.as_str())
}

/// The asks sent so far, by kind and id.
fn sent(asks: &mpsc::Receiver<Ask>) -> Vec<String> {
    asks.try_iter()
        .map(|a| match a {
            Ask::Open(id) => format!("open {id}"),
            Ask::Trash { id, back } => format!("trash {id} {back}"),
            Ask::Relabel { id, add, remove } => format!("relabel {id} +{add:?} -{remove:?}"),
            _ => "other".into(),
        })
        .collect()
}

#[test]
fn d_in_the_reader_opens_the_next_older_thread_read_and_selected() {
    let (mut p, asks, _a) = reading(&["a", "b", "c", "d", "e"], 1);
    press(&mut p, "d");
    assert_eq!(open_id(&p), Some("c"), "the thread that took b's place");
    assert_eq!(p.sel, 1, "and the list's selection with it");
    assert!(!p.rows[1].unread, "opened, so read, as Gmail does");
    let got = sent(&asks);
    assert!(got.contains(&"open c".to_string()), "{got:?}");
    assert!(got.contains(&"trash b false".to_string()), "{got:?}");
    assert!(
        p.note.starts_with("deleted \u{201c}about b\u{201d}"),
        "{}",
        p.note
    );
}

#[test]
fn archive_advances_too_the_last_opens_the_one_above_and_the_only_one_ends_on_the_list() {
    let (mut p, asks, _a) = reading(&["a", "b", "c"], 1);
    press(&mut p, "e");
    assert_eq!(open_id(&p), Some("c"));
    assert!(sent(&asks).contains(&"relabel b +[] -[\"INBOX\"]".to_string()));
    let (mut last, _asks, _a) = reading(&["a", "b", "c"], 2);
    press(&mut last, "d");
    assert_eq!((open_id(&last), last.sel), (Some("b"), 1), "the one above");
    let (mut only, _asks, _a) = reading(&["a"], 0);
    press(&mut only, "d");
    assert_eq!(open_id(&only), None, "back on the list");
    assert!(only.rows.is_empty());
}

#[test]
fn a_late_answer_for_the_deleted_thread_never_fills_the_next_one() {
    let (mut p, _asks, answer) = reading(&["a", "b", "c"], 0);
    press(&mut p, "d");
    answer
        .send(Got::Opened("a".into(), vec![Mail::default()]))
        .unwrap();
    p.poll();
    assert_eq!(open_id(&p), Some("b"));
    assert!(
        p.reading.as_ref().unwrap().mails.is_none(),
        "b is still coming"
    );
}

#[test]
fn z_after_an_advance_brings_the_thread_back_and_opens_it_again() {
    let (mut p, asks, _a) = reading(&["a", "b", "c"], 1);
    press(&mut p, "d");
    let _ = sent(&asks);
    press(&mut p, "z");
    assert_eq!(open_id(&p), Some("b"), "the deleted thread, open again");
    assert_eq!(p.sel, 1);
    let ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c"]);
    let got = sent(&asks);
    assert!(
        got.contains(&"trash b true".to_string()) && got.contains(&"open b".to_string()),
        "{got:?}"
    );
    assert!(!p.rows[2].unread, "c stays read: it was opened");
}

#[test]
fn from_the_list_a_delete_stays_on_the_list() {
    let (mut p, _asks, _a) = reading(&["a", "b", "c"], 1);
    p.reading = None;
    press(&mut p, "d");
    assert_eq!(open_id(&p), None);
    press(&mut p, "z");
    assert_eq!(open_id(&p), None, "undo puts it back without opening it");
}

#[test]
fn n_and_p_in_the_reader_open_the_next_and_the_previous_thread_in_place() {
    let (mut p, asks, _) = reading(&["a", "b", "c"], 1);
    press(&mut p, "n");
    assert_eq!(
        (open_id(&p), p.sel),
        (Some("c"), 2),
        "n: older, the row below"
    );
    press(&mut p, "J");
    assert_eq!(open_id(&p), Some("c"), "past the oldest it stays (J is n)");
    assert_eq!(p.note, "this is the end of the list");
    press(&mut p, "p");
    press(&mut p, "K");
    assert_eq!((open_id(&p), p.sel), (Some("a"), 0), "K is p");
    press(&mut p, "p");
    assert_eq!(p.note, "this is the top of the list");
    let opened: Vec<String> = sent(&asks)
        .into_iter()
        .filter(|a| a.starts_with("open"))
        .collect();
    assert_eq!(opened, ["open c", "open b", "open a"]);
    assert!(
        p.rows.iter().all(|t| !t.unread),
        "each marked read as it opened"
    );
}

/// Down and Up in the reader are n and p; j and k still scroll the mail.
#[test]
fn the_arrows_in_the_reader_open_the_next_and_the_previous_thread() {
    use winit::keyboard::NamedKey::{ArrowDown, ArrowUp};
    let (mut p, _asks, _) = reading(&["a", "b", "c"], 1);
    p.press(&Key::Named(ArrowDown), 80, 24);
    assert_eq!((open_id(&p), p.sel), (Some("c"), 2), "down: the row below");
    p.press(&Key::Named(ArrowUp), 80, 24);
    p.press(&Key::Named(ArrowUp), 80, 24);
    assert_eq!((open_id(&p), p.sel), (Some("a"), 0), "up: the row above");
    press(&mut p, "j");
    assert_eq!(open_id(&p), Some("a"), "j scrolls, never leaves the thread");
}

#[test]
fn a_long_thread_folds_its_older_mails_to_a_line_until_a_click_or_x_opens_them() {
    let (mut p, _asks, _) = reading(&["a"], 0);
    let mail = |from: &str, words: &str| Mail {
        from: from.into(),
        date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
        rich: crate::inbox::rich::Rich::of_plain(words, None),
        ..Mail::default()
    };
    p.reading.as_mut().unwrap().mails = Some(vec![
        mail("Ann <a@x.org>", "First words"),
        mail("Bob <b@y.org>", "Second words"),
        mail("Cy <c@z.org>", "Newest words"),
    ]);
    let laid = |p: &InboxPane| p.reading.as_ref().unwrap().laid(80);
    assert_eq!(laid(&p).folded.len(), 2, "all but the newest");
    let text: String = laid(&p).lines.iter().flatten().map(|c| c.c).collect();
    assert!(text.contains("Newest words") && text.contains("Second words"));
    let row = laid(&p).folded[1].0 as u16;
    assert!(p.unfold_at(84, row), "a click on Bob's line");
    assert_eq!(laid(&p).folded.len(), 1);
    press(&mut p, "x");
    assert!(laid(&p).folded.is_empty(), "x: every mail open");
    press(&mut p, "x");
    assert_eq!(laid(&p).folded.len(), 2, "x again folds them");
    assert_eq!(
        crate::inbox::reader::text_width(300),
        crate::inbox::reader::MEASURE
    );
}
