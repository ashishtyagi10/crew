use std::sync::mpsc;

use super::failure;
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Got, Worker};
use crate::inbox::{InboxPane, Stage};

fn pane() -> (InboxPane, mpsc::Sender<Got>) {
    let (tx, _) = mpsc::channel();
    let (answer, rx) = mpsc::channel();
    (InboxPane::with(Stage::Ready(Worker { tx, rx })), answer)
}

/// The words drawn above the foot, the list card's edge left out.
fn words(p: &InboxPane, cols: u16, rows: u16) -> String {
    let cells = p.cells(cols, rows);
    let edge = |c: char| ('\u{2500}'..='\u{257f}').contains(&c);
    (0..p.body_rows(cols, rows))
        .map(|r| {
            let mut line = vec![' '; usize::from(cols)];
            for c in cells.iter().filter(|c| c.row == r && !edge(c.c)) {
                line[usize::from(c.col)] = c.c;
            }
            line.into_iter().collect::<String>().trim().to_string()
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_list_gmail_would_not_give_says_what_in_words_and_the_key_that_mends_it() {
    let (mut p, answer) = pane();
    let raw = "could not reach Gmail: error sending request for url (dns error)";
    answer.send(Got::Failed(raw.into())).unwrap();
    p.poll();
    let said = words(&p, 70, 20);
    assert!(
        said.contains("Gmail can\u{2019}t be reached from here"),
        "{said}"
    );
    assert!(said.contains("R tries again"), "{said}");
    assert!(
        !said.contains("dns") && p.note.is_empty(),
        "never the raw error"
    );
    answer
        .send(Got::Page {
            query: String::new(),
            rows: vec![Thread::default()],
            next: None,
            more: false,
        })
        .unwrap();
    p.poll();
    assert!(p.failed.is_none(), "a page that lands ends it");
    answer
        .send(Got::Failed("Gmail 500: backend".into()))
        .unwrap();
    p.poll();
    assert!(p.failed.is_none(), "with rows to show, it is a note");
    assert_eq!(p.note, "Gmail 500: backend");
}

#[test]
fn failures_read_as_people_would_say_them() {
    let said = |e: &str| failure(e).0;
    assert_eq!(
        said("Gmail refused the sign-in (401)"),
        "Gmail turned down crew\u{2019}s sign-in"
    );
    assert_eq!(
        failure("Gmail 403: Request had insufficient authentication scopes.").1,
        "/inbox signout, then Enter, signs in again"
    );
    assert_eq!(
        said("Gmail 429: Rate Limit Exceeded"),
        "Gmail asked crew to slow down"
    );
    assert_eq!(said("Gmail 503: unavailable"), "Gmail is having trouble");
    assert_eq!(
        said("something else"),
        "Gmail wouldn\u{2019}t list the inbox"
    );
}

#[test]
fn loading_draws_rows_in_outline_and_an_empty_inbox_a_mark_and_when_it_was_checked() {
    let _g = crate::app::theme_test_guard();
    let (mut p, answer) = pane();
    p.busy = true;
    assert_eq!(words(&p, 60, 20), "", "no words while loading");
    let outline = p.art(60, 20, 2.0).1;
    assert!(outline.len() >= 3 * 9, "rows in outline: {}", outline.len());
    p.busy = false;
    answer.send(Got::Checked).unwrap();
    p.poll();
    let said = words(&p, 60, 20);
    assert!(said.contains("inbox zero"), "{said}");
    assert!(said.contains("checked just now"), "{said}");
    let mark = p.art(60, 20, 2.0).1;
    assert!(!mark.is_empty(), "a drawn mark");
    let accent = crate::palette::accent();
    assert!(mark.iter().all(|q| q.color == accent));
    p.query = "from:ann".into();
    let said = words(&p, 60, 20);
    assert!(
        said.contains("no mail matches \u{201c}from:ann\u{201d}"),
        "{said}"
    );
    assert!(said.contains("esc goes back to the inbox"), "{said}");
    assert!(p.art(60, 20, 2.0).1.is_empty(), "no mark for a search");
}
