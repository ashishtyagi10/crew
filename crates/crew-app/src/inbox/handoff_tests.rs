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
    assert_eq!(p.then, Some((Then::Ask, "t1".to_string())));
    assert!(p.to_smith.is_none(), "nothing to hand over yet");
    answer.send(Got::Opened("t1".into(), mails())).unwrap();
    p.poll();
    let sent = p.to_smith.take().expect("handed over");
    assert!(sent.prompt.contains("<<<MAIL") && sent.prompt.contains("by Friday"));
    assert!(sent.shown.contains("Q4 review") && !sent.draft);
}

#[test]
fn smiths_draft_for_a_closed_box_is_let_go_never_saved() {
    let (mut p, asks, _answer) = pane();
    p.reading = Some(super::Reading {
        mails: Some(mails()),
        ..super::Reading::new(p.rows[0].clone())
    });
    // Asked for from the box, then the box closed: the draft is let go,
    // never saved to Gmail behind your back.
    p.act(super::Then::Draft);
    let sent = p.to_smith.take().expect("handed over");
    assert!(sent.draft, "a draft of the box's words");
    p.waiting = Some(super::Waiting { since: 0, at_ms: 0 });
    p.drafted_by_smith(Some("Sure, Friday.".into()));
    assert!(asks.try_iter().next().is_none(), "nothing asked of Gmail");
    assert!(p.note.contains("let go"), "{}", p.note);
    p.waiting = Some(super::Waiting { since: 0, at_ms: 0 });
    p.drafted_by_smith(None);
    assert!(
        p.note.contains("tab in the reply box asks again"),
        "{}",
        p.note
    );
}

#[test]
fn t_adds_a_todo_row_with_the_deadline_the_mail_states() {
    let _g = crate::todopane::store::test_guard(Vec::new());
    let (mut p, _asks, _answer) = pane();
    p.reading = Some(super::Reading {
        mails: Some(mails()),
        ..super::Reading::new(p.rows[0].clone())
    });
    p.press(&Key::Character("t".into()), 80, 24);
    assert!(
        crate::todopane::store::snapshot().is_empty(),
        "a card first"
    );
    p.press(&Key::Named(winit::keyboard::NamedKey::Enter), 80, 24);
    let items = crate::todopane::store::snapshot();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Reply to Ann: Q4 review");
    assert!(items[0].due_ms.is_some(), "\"by Friday\" is the due date");
    assert!(p.note.starts_with("added to /todo"), "{}", p.note);
}

#[test]
fn a_key_waiting_for_a_thread_never_acts_on_another() {
    let (mut p, _asks, _answer) = pane();
    p.rows.push(Thread {
        id: "t2".into(),
        subject: "Other".into(),
        ..Thread::default()
    });
    p.press(&Key::Character("r".into()), 80, 24);
    assert!(p.then.is_some(), "waiting for t1's mails");
    p.press(&Key::Named(winit::keyboard::NamedKey::Escape), 80, 24);
    assert!(p.open_at(1), "another thread");
    p.take(crate::inbox::worker::Got::Opened("t2".into(), mails()));
    assert!(
        p.compose.is_none(),
        "no reply box on a thread r was not pressed on"
    );
    assert!(p.then.is_none(), "and the stale key is let go");
}
