use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use super::keys::InboxAction;
use super::wire::Thread;
use super::worker::{Ask, Got, Worker};
use super::{InboxPane, Stage};

/// A signed-in pane over a hand-held worker: what the pane asks, and a way
/// to answer it.
fn ready() -> (InboxPane, mpsc::Receiver<Ask>, mpsc::Sender<Got>) {
    let (tx, asks) = mpsc::channel();
    let (answer, rx) = mpsc::channel();
    (
        InboxPane::with(Stage::Ready(Worker { tx, rx })),
        asks,
        answer,
    )
}

fn thread(id: &str, unread: bool) -> Thread {
    Thread {
        id: id.into(),
        from: format!("{id} <{id}@x.org>"),
        subject: format!("about {id}"),
        unread,
        count: 1,
        ..Thread::default()
    }
}

fn page(query: &str, ids: &[&str], more: bool) -> Got {
    Got::Page {
        query: query.into(),
        rows: ids.iter().map(|i| thread(i, true)).collect(),
        next: Some("n".into()),
        more,
    }
}

fn ch(c: &str) -> Key {
    Key::Character(c.into())
}

#[test]
fn a_page_lands_keeps_the_selected_thread_and_a_stale_search_is_dropped() {
    let (mut p, _asks, answer) = ready();
    for g in [page("", &["a", "b", "c"], false), Got::Unread(3)] {
        answer.send(g).unwrap();
    }
    assert!(p.poll());
    assert_eq!(
        (p.rows.len(), p.legend()),
        (3, "inbox \u{b7} 3 unread".to_string())
    );
    p.sel = 2;
    answer
        .send(page("", &["new", "a", "b", "c"], false))
        .unwrap();
    answer.send(page("from:old", &["z"], false)).unwrap();
    p.poll();
    assert_eq!(p.rows[p.sel].id, "c", "the selection follows its thread");
    assert_eq!(p.rows.len(), 4, "a page for an older search does not land");
}

#[test]
fn enter_reads_the_thread_and_esc_steps_back_then_closes() {
    let (mut p, asks, answer) = ready();
    answer.send(page("", &["a", "b", "c"], false)).unwrap();
    p.poll();
    p.press(&Key::Named(NamedKey::ArrowDown), 80, 24);
    p.press(&Key::Named(NamedKey::Enter), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::Open(id)) if id == "b"));
    assert_eq!(p.reading.as_ref().map(|r| r.thread.id.as_str()), Some("b"));
    answer.send(Got::Opened("b".into(), vec![])).unwrap();
    p.poll();
    assert!(p.reading.as_ref().unwrap().mails.is_some());
    p.press(&Key::Named(NamedKey::Escape), 80, 24);
    assert!(p.reading.is_none(), "back to the list");
    assert!(matches!(
        p.press(&Key::Named(NamedKey::Escape), 80, 24),
        Some(InboxAction::Close)
    ));
}

#[test]
fn slash_searches_with_gmails_own_syntax_and_esc_clears_it() {
    let (mut p, asks, _answer) = ready();
    p.press(&ch("/"), 80, 24);
    for c in ["f", "r", "o", "m", ":", "a", "n", "n"] {
        p.press(&ch(c), 80, 24);
    }
    p.press(&Key::Named(NamedKey::Enter), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::List(q)) if q == "from:ann"));
    assert_eq!(p.legend(), "inbox \u{b7} from:ann");
    p.press(&Key::Named(NamedKey::Escape), 80, 24);
    assert!(
        matches!(asks.try_recv(), Ok(Ask::List(q)) if q.is_empty()),
        "esc goes back to the inbox"
    );
}

#[test]
fn the_last_row_asks_for_the_next_page_once() {
    let (mut p, asks, answer) = ready();
    answer.send(page("", &["a", "b"], false)).unwrap();
    p.poll();
    p.press(&Key::Named(NamedKey::ArrowDown), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::More)));
    p.press(&Key::Named(NamedKey::ArrowDown), 80, 24);
    assert!(asks.try_recv().is_err(), "busy: not asked twice");
    answer.send(page("", &["c"], true)).unwrap();
    p.poll();
    assert_eq!(p.rows.len(), 3, "the next page is added, not swapped in");
}

#[test]
fn an_ended_grant_goes_back_to_setup_and_says_how_to_keep_it() {
    let (mut p, _asks, answer) = ready();
    answer
        .send(Got::Ended(
            "invalid_grant: Token has been expired or revoked.".into(),
        ))
        .unwrap();
    p.poll();
    assert!(matches!(p.stage, Stage::Setup { .. }));
    assert!(p.note.contains("Publish app"), "{}", p.note);
}

#[test]
fn a_pasted_client_is_the_one_signed_in_with() {
    let mut p = InboxPane::with(Stage::Setup { found: None });
    p.paste(r#"{"installed":{"client_id":"1.apps.googleusercontent.com","client_secret":"s"}}"#);
    assert!(
        matches!(&p.stage, Stage::Setup { found: Some((_, c)) } if c.id == "1.apps.googleusercontent.com")
    );
    assert_eq!(p.glance(), "not signed in");
}

#[test]
fn the_list_draws_unread_dots_senders_and_the_hint() {
    let _g = crate::app::theme_test_guard();
    let (mut p, _asks, answer) = ready();
    answer.send(page("", &["ann", "bo"], false)).unwrap();
    p.poll();
    let cells = p.cells(60, 6);
    let row = |r: u16| -> String {
        let mut line = vec![' '; 60];
        for c in cells.iter().filter(|c| c.row == r) {
            line[c.col as usize] = c.c;
        }
        line.into_iter().collect()
    };
    assert_eq!(row(0).trim(), "undated", "the day's band, first");
    assert!(
        row(1).starts_with(" \u{25cf} ann  about ann"),
        "{:?}",
        row(1)
    );
    assert!(row(5).contains("enter read"), "{:?}", row(5));
    assert!(
        cells.iter().all(|c| c.col < 59),
        "nothing in the margin column"
    );
}
