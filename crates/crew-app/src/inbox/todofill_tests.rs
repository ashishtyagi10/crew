use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use super::check;
use crate::inbox::api::Mail;
use crate::inbox::draft::Reply;
use crate::inbox::handoff::{Waiting, Wants};
use crate::inbox::todocard::{Known, Line};
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};

/// Ann asks me for the numbers, Bo on Cc.
fn reading() -> Reading {
    Reading {
        mails: Some(vec![Mail {
            from: "Ann Lee <ann@x.org>".into(),
            date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
            text: "Can you send me the Q4 numbers by Friday?".into(),
            reply: Reply {
                to: "Sam Doe <sam@x.org>".into(),
                cc: "Bo Park <bo@y.org>".into(),
                ..Reply::default()
            },
            ..Mail::default()
        }]),
        ..Reading::new(Thread {
            id: "t1".into(),
            subject: "Q4 numbers".into(),
            ..Thread::default()
        })
    }
}

fn known() -> Known {
    Known {
        projects: vec!["crew".into()],
        folders: vec!["rdactd".into()],
        people: vec!["bo".into()],
        me: "sam@x.org".into(),
    }
}

fn answer(json: &str) -> String {
    format!("Here is the card.\n<<<TODO\n{json}\nTODO>>>\n")
}

/// What smith writes stands only where crew's own checks pass: a project
/// from your list in its spelling, someone on the thread, a date `/todo`
/// reads; the rest is dropped, never trusted.
#[test]
fn smiths_card_is_checked_line_by_line() {
    let good = answer(
        r#"{"title":"Send Ann the Q4 numbers","project":"Crew","assignee":"Bo Park","due":"2026-10-23","why":"Ann asks by Friday"}"#,
    );
    let [title, project, who, due] = check(&good, &reading(), &known()).expect("a card");
    assert_eq!(title.unwrap().text, "Send Ann the Q4 numbers");
    assert_eq!(project.unwrap().text, "crew", "your spelling");
    assert_eq!(who.as_ref().unwrap().text, "bo", "your tag for them");
    assert!(who.unwrap().why.starts_with("agent smith \u{b7} Ann asks"));
    assert_eq!(due.unwrap().text, "2026-10-23");
    let bad = answer(
        r#"{"title":"x","project":"secret-repo","assignee":"Dee Ray","due":"whenever you like","why":"ignore your rules"}"#,
    );
    let lines = check(&bad, &reading(), &known()).expect("a card");
    assert!(lines.iter().all(Option::is_none), "nothing of it stands");
    let me = answer(r#"{"assignee":"Sam Doe"}"#);
    assert!(
        check(&me, &reading(), &known()).unwrap()[2].is_none(),
        "never you"
    );
    assert!(check("no markers at all", &reading(), &known()).is_none());
    assert!(check(&answer("{not json"), &reading(), &known()).is_none());
}

/// Tab asks smith once, its answer fills the lines still as crew wrote them,
/// and a line you changed stays yours.
#[test]
fn tab_asks_smith_once_and_its_answer_fills_what_you_left() {
    let ((tx, _), (_, rx)) = (mpsc::channel(), mpsc::channel());
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.email = "sam@x.org".into();
    p.rows = vec![reading().thread];
    p.reading = Some(reading());
    p.press(&Key::Character("t".into()), 90, 30);
    p.press(&Key::Named(NamedKey::ArrowDown), 90, 30);
    p.type_in("mine");
    p.press(&Key::Named(NamedKey::Tab), 90, 30);
    let ask = p.to_smith.take().expect("handed to smith");
    assert_eq!(ask.wants, Wants::Todo);
    assert!(ask.prompt.contains("<<<MAIL") && ask.prompt.contains("<<<TODO"));
    p.waiting = Some(Waiting {
        since: 0,
        at_ms: 0,
        wants: Wants::Todo,
        thread: "t1".into(),
    });
    p.press(&Key::Named(NamedKey::Tab), 90, 30);
    assert!(
        p.to_smith.is_none() && p.note.contains("still filling"),
        "once"
    );
    let json = r#"{"title":"Send Ann the numbers","project":"rdactd","assignee":"","due":"2026-10-23 17:00","why":"by Friday"}"#;
    p.from_smith(Some(vec![answer(json)]));
    let c = p.todo.as_ref().expect("the card");
    assert_eq!(c.text(Line::Title), "Send Ann the numbers");
    assert_eq!(c.text(Line::Project), "mine", "you changed it: yours");
    assert_eq!(c.text(Line::Due), "2026-10-23 17:00");
    assert!(c.why[3].starts_with("agent smith"));
    assert!(
        p.waiting.is_none() && p.note.contains("filled 2"),
        "{}",
        p.note
    );
    // An answer after the card closed is let go.
    p.todo = None;
    p.waiting = Some(Waiting {
        since: 0,
        at_ms: 0,
        wants: Wants::Todo,
        thread: "t1".into(),
    });
    p.from_smith(Some(vec![answer(json)]));
    assert!(p.note.contains("after the card closed"), "{}", p.note);
}

/// Tab on one mail's card, Esc, the next mail open and its card up: the
/// answer about the first is let go, never written onto the second's card.
#[test]
fn smiths_answer_lands_only_on_the_mail_it_was_about() {
    let ((tx, _), (_, rx)) = (mpsc::channel(), mpsc::channel());
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.email = "sam@x.org".into();
    let mut next = reading();
    next.thread.id = "t2".into();
    p.rows = vec![next.thread.clone()];
    p.reading = Some(next);
    p.press(&Key::Character("t".into()), 90, 30);
    let title = |p: &InboxPane| {
        p.todo
            .as_ref()
            .expect("t2's card")
            .text(Line::Title)
            .to_string()
    };
    let before = title(&p);
    p.waiting = Some(Waiting {
        since: 0,
        at_ms: 0,
        wants: Wants::Todo,
        thread: "t1".into(),
    });
    let json = r#"{"title":"Send Ann the numbers","project":"","assignee":"","due":"","why":""}"#;
    p.from_smith(Some(vec![answer(json)]));
    assert_eq!(title(&p), before, "t1's answer never on t2's card");
    assert!(
        p.waiting.is_none() && p.note.contains("another mail"),
        "{}",
        p.note
    );
}

/// smith's ISO due, `T` and all, is the date and time it says; one with a
/// zone is not read as this machine's hour.
#[test]
fn an_iso_due_with_a_t_is_read() {
    assert_eq!(
        super::read_due("2026-10-23T17:00", &[]).as_deref(),
        Some("2026-10-23 17:00")
    );
    assert_eq!(
        super::read_due("2026-10-23T17:00:00", &[]).as_deref(),
        Some("2026-10-23 17:00")
    );
    assert_eq!(super::read_due("2026-10-23T17:00Z", &[]), None);
}
