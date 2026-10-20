use std::sync::mpsc;

use winit::keyboard::Key;

use super::super::api::Mail;
use super::super::draft::Reply;
use super::super::wire::Thread;
use super::super::worker::{Ask, Got, Worker};
use super::super::{InboxPane, Stage};
use super::Then;

fn pane() -> (InboxPane, mpsc::Receiver<Ask>, mpsc::Sender<Got>) {
    let (tx, asks) = mpsc::channel();
    let (answer, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = vec![Thread {
        id: "t1".into(),
        subject: "Q4 review".into(),
        ..Thread::default()
    }];
    p.email = "me@gmail.com".into();
    (p, asks, answer)
}

fn mails() -> Vec<Mail> {
    let reply = Reply {
        subject: "Q4 review".into(),
        message_id: "<1@x>".into(),
        ..Reply::default()
    };
    vec![Mail {
        from: "Ann <ann@x.org>".into(),
        text: "Can you send it by Friday?".into(),
        reply,
        ..Mail::default()
    }]
}

#[test]
fn a_in_the_list_opens_the_thread_and_hands_it_over_when_it_lands() {
    let (mut p, asks, answer) = pane();
    p.press(&Key::Character("a".into()), 80, 24);
    assert!(matches!(asks.try_recv(), Ok(Ask::Open(id)) if id == "t1"));
    assert_eq!(p.then, Some(Then::Ask));
    assert!(p.to_smith.is_none(), "nothing to hand over yet");
    answer.send(Got::Opened("t1".into(), mails())).unwrap();
    p.poll();
    let sent = p.to_smith.take().expect("handed over");
    assert!(sent.prompt.contains("<<<MAIL") && sent.prompt.contains("by Friday"));
    assert!(sent.shown.contains("Q4 review") && sent.draft.is_none());
}

#[test]
fn d_asks_for_a_reply_and_smiths_body_becomes_a_draft_in_the_thread() {
    let (mut p, asks, _answer) = pane();
    p.reading = Some(super::Reading {
        thread: p.rows[0].clone(),
        mails: Some(mails()),
        scroll: 0,
    });
    p.press(&Key::Character("d".into()), 80, 24);
    let sent = p.to_smith.take().expect("handed over");
    let answer = sent.draft.expect("the draft's headers");
    assert_eq!(
        (answer.to.as_str(), answer.subject.as_str()),
        ("Ann <ann@x.org>", "Re: Q4 review")
    );
    p.waiting = Some(super::Waiting {
        answer,
        since: 0,
        at_ms: 0,
    });
    p.drafted_by_smith(Some("Sure, Friday.".into()));
    match asks.try_recv() {
        Ok(Ask::Draft {
            thread_id, subject, ..
        }) => assert_eq!(
            (thread_id.as_str(), subject.as_str()),
            ("t1", "Re: Q4 review")
        ),
        _ => panic!("no draft asked"),
    }
    p.waiting = Some(super::Waiting {
        answer: Default::default(),
        since: 0,
        at_ms: 0,
    });
    p.drafted_by_smith(None);
    assert!(p.note.contains("held no draft"), "{}", p.note);
}

#[test]
fn t_adds_a_todo_row_with_the_deadline_the_mail_states() {
    let _g = crate::todopane::store::test_guard(Vec::new());
    let (mut p, _asks, _answer) = pane();
    p.reading = Some(super::Reading {
        thread: p.rows[0].clone(),
        mails: Some(mails()),
        scroll: 0,
    });
    p.press(&Key::Character("t".into()), 80, 24);
    let items = crate::todopane::store::snapshot();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Reply to Ann: Q4 review");
    assert!(items[0].due_ms.is_some(), "\"by Friday\" is the due date");
    assert!(p.note.starts_with("added to /todo"), "{}", p.note);
}
