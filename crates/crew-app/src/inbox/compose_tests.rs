use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use crate::inbox::api::Mail;
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Got, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

/// A pane reading Ann's mail, and what it asks of Gmail.
fn reading() -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    let thread = Thread {
        id: "t1".into(),
        subject: "Lunch".into(),
        ..Thread::default()
    };
    p.rows = vec![thread.clone()];
    p.email = "me@y.org".into();
    let mut mail = Mail {
        from: "Ann Lee <ann@x.org>".into(),
        ..Mail::default()
    };
    mail.reply.subject = "Lunch".into();
    p.reading = Some(Reading {
        mails: Some(vec![mail]),
        ..Reading::new(thread)
    });
    (p, asks)
}

fn press(p: &mut InboxPane, k: Key) {
    p.press(&k, 80, 30);
}

fn typed(p: &mut InboxPane, s: &str) {
    for c in s.chars() {
        press(p, Key::Character(c.to_string().into()));
    }
}

fn text(p: &InboxPane) -> String {
    p.compose
        .as_ref()
        .map(|c| c.body.text.clone())
        .unwrap_or_default()
}

#[test]
fn r_opens_a_box_whose_keys_are_its_own_and_esc_twice_lets_words_go() {
    let (mut p, asks) = reading();
    typed(&mut p, "r");
    let c = p.compose.as_ref().expect("the box");
    assert_eq!(
        (c.answer.to.as_str(), c.answer.subject.as_str()),
        ("Ann Lee <ann@x.org>", "Re: Lunch")
    );
    typed(&mut p, "dear");
    press(&mut p, Key::Named(NamedKey::Enter));
    typed(&mut p, "Ann!");
    press(&mut p, Key::Named(NamedKey::Backspace));
    assert_eq!(text(&p), "dear\nAnn", "d is a letter here, not delete");
    assert!(asks.try_iter().all(|a| !matches!(a, Ask::Trash { .. })));
    press(&mut p, Key::Named(NamedKey::Escape));
    assert!(p.compose.is_some() && p.note.contains("esc again"));
    press(&mut p, Key::Named(NamedKey::Escape));
    assert!(p.compose.is_none());
    let k = p.cards(80, 30);
    assert!(k.reply.is_none(), "the box is gone");
    assert_eq!(
        k.mail.unwrap().h,
        p.body_rows(80, 30),
        "the mail has it all"
    );
}

#[test]
fn a_reply_is_sent_in_its_thread_and_stays_until_gmail_says_it_went() {
    let (mut p, asks) = reading();
    typed(&mut p, "r");
    p.send_reply();
    assert_eq!(p.note, "write something first");
    typed(&mut p, "Friday works.");
    let cells = p.cells(80, 30);
    assert!(cells.iter().any(|c| c.c == '\u{256d}'), "the box is drawn");
    p.send_reply();
    let sent = asks.try_iter().find_map(|a| match a {
        Ask::Send { thread_id, raw, to } => Some((thread_id, raw, to)),
        _ => None,
    });
    let (thread_id, raw, to) = sent.expect("sent");
    assert_eq!(
        (thread_id.as_str(), to.as_str()),
        ("t1", "Ann Lee <ann@x.org>")
    );
    assert!(!raw.is_empty());
    assert!(p.compose.as_ref().is_some_and(|c| c.sending));
    p.take(Got::Failed("network down".into()));
    assert_eq!(text(&p), "Friday works.", "a failure keeps the words");
    assert!(!p.compose.as_ref().unwrap().sending);
    p.take(Got::Sent(to));
    assert!(p.compose.is_none());
    assert_eq!(p.note, "sent to Ann Lee");
    assert!(
        asks.try_iter()
            .any(|a| matches!(a, Ask::Open(id) if id == "t1")),
        "the thread again"
    );
}

#[test]
fn tab_asks_agent_smith_and_its_draft_lands_in_the_box() {
    let (mut p, _) = reading();
    typed(&mut p, "r");
    press(&mut p, Key::Named(NamedKey::Tab));
    let ask = p.to_smith.take().expect("smith asked");
    assert!(ask.draft, "a draft of the box's words");
    p.waiting = Some(crate::inbox::handoff::Waiting { since: 0, at_ms: 0 });
    p.drafted_by_smith(Some("Friday is good, see you then.".into()));
    assert_eq!(text(&p), "Friday is good, see you then.");
    p.paste(" Ann");
    assert!(
        text(&p).ends_with("then. Ann"),
        "a paste lands at the caret"
    );
}
