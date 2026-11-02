//! `c` in the list: a new mail, to, cc, subject and words, sent on no thread.
use std::sync::mpsc;

use base64::Engine;
use winit::keyboard::{Key, NamedKey};

use crate::inbox::compose::{Kind, On};
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Worker};
use crate::inbox::{InboxPane, Stage};

/// A pane on the list, one thread in it, and what it asks of Gmail.
fn listing() -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = vec![Thread {
        id: "t1".into(),
        subject: "Lunch".into(),
        ..Thread::default()
    }];
    p.email = "me@y.org".into();
    (p, asks)
}

fn press(p: &mut InboxPane, k: Key) {
    p.press(&k, 80, 30);
}

fn named(p: &mut InboxPane, k: NamedKey) {
    press(p, Key::Named(k));
}

fn typed(p: &mut InboxPane, s: &str) {
    for c in s.chars() {
        let k = match c {
            ' ' => Key::Named(NamedKey::Space),
            c => Key::Character(c.to_string().into()),
        };
        press(p, k);
    }
}

fn on(p: &InboxPane) -> On {
    p.compose.as_ref().expect("the box").on
}

#[test]
fn c_in_the_list_opens_a_new_mail_on_its_to_line_and_its_keys_are_its_own() {
    let (mut p, asks) = listing();
    typed(&mut p, "c");
    let c = p.compose.as_ref().expect("the box");
    assert_eq!((c.kind, c.on), (Kind::New, On::To));
    assert!(p.reading.is_none(), "no thread opened");
    // `e` and `d` are letters in the box, not an archive or a delete.
    typed(&mut p, "ed@x.org");
    assert_eq!(p.compose.as_ref().unwrap().to.text, "ed@x.org");
    assert!(asks.try_iter().all(|a| !matches!(a, Ask::Open(_))));
    // Tab: to → cc → subject → words; ↑ from the words is the subject.
    for want in [On::Cc, On::Subject, On::Body] {
        named(&mut p, NamedKey::Tab);
        assert_eq!(on(&p), want);
    }
    named(&mut p, NamedKey::ArrowUp);
    assert_eq!(on(&p), On::Subject);
    // Untouched but for the address: one Esc warns, a second lets it go.
    named(&mut p, NamedKey::Escape);
    assert!(p.compose.is_some());
    named(&mut p, NamedKey::Escape);
    assert!(p.compose.is_none());
    typed(&mut p, "c");
    named(&mut p, NamedKey::Escape);
    assert!(p.compose.is_none(), "an empty box goes at once");
}

#[test]
fn a_new_mail_goes_on_its_own_subject_and_no_thread() {
    let (mut p, asks) = listing();
    typed(&mut p, "c");
    typed(&mut p, "Ed <ed@x.org>");
    named(&mut p, NamedKey::Enter);
    named(&mut p, NamedKey::Enter);
    typed(&mut p, "Plans");
    named(&mut p, NamedKey::Enter);
    typed(&mut p, "see you at noon");
    p.send_reply();
    let sent = asks.try_iter().find_map(|a| match a {
        Ask::Send { thread_id, raw, to } => Some((thread_id, raw, to)),
        _ => None,
    });
    let (thread, raw, to) = sent.expect("sent");
    assert_eq!((thread.as_str(), to.as_str()), ("", "Ed <ed@x.org>"));
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .unwrap();
    let m = String::from_utf8(bytes).unwrap();
    assert!(m.contains("Subject: Plans\r\n"), "{m}");
    assert!(!m.contains("In-Reply-To"), "{m}");
    let c = p.compose.as_ref().expect("kept until Gmail says it went");
    assert_eq!((c.body.text.as_str(), c.sending), ("see you at noon", true));
}

#[test]
fn a_new_mail_with_no_address_is_refused_on_its_to_line() {
    let (mut p, asks) = listing();
    typed(&mut p, "c");
    named(&mut p, NamedKey::Tab);
    named(&mut p, NamedKey::Tab);
    named(&mut p, NamedKey::Tab);
    typed(&mut p, "hi");
    p.send_reply();
    assert_eq!(on(&p), On::To);
    assert!(asks.try_iter().all(|a| !matches!(a, Ask::Send { .. })));
}

#[test]
fn the_new_mail_sits_under_the_list_with_a_subject_line_and_rows_stay_held() {
    let (mut p, _asks) = listing();
    typed(&mut p, "c");
    let k = p.cards(80, 30);
    let (list, reply) = (k.list.expect("the list"), k.reply.expect("the box"));
    assert_eq!(
        (reply.col, reply.w),
        (list.col, list.w),
        "as wide as the list"
    );
    assert_eq!(reply.row, list.row + list.h, "right under it");
    let words: String = p.cells(80, 30).iter().map(|c| c.c).collect();
    for want in ["new mail", "subject", "who it goes to"] {
        assert!(words.contains(want), "{want} drawn");
    }
    // A thread asked for while it is written: the mail stays, unsaid.
    assert!(!p.open_at(0));
    assert!(p.reading.is_none());
    assert!(p.note.contains("being written"), "{}", p.note);
}

#[test]
fn the_foot_names_the_subject_line_over_a_new_mails_words() {
    let (mut p, _asks) = listing();
    typed(&mut p, "c");
    p.compose.as_mut().unwrap().on = On::Body;
    let keys = p.compose_keys();
    assert!(keys.iter().any(|k| k.1 == "to, cc, subject"));
    assert!(keys.iter().any(|k| k.0 == "tab"), "smith drafts it");
}

#[test]
fn tab_in_a_new_mail_asks_smith_and_its_draft_replaces_the_notes() {
    let (mut p, _asks) = listing();
    typed(&mut p, "c");
    // Nothing to go on yet: smith is not asked.
    p.compose.as_mut().unwrap().on = On::Body;
    named(&mut p, NamedKey::Tab);
    assert!(p.to_smith.is_none(), "{}", p.note);
    p.compose.as_mut().unwrap().on = On::To;
    typed(&mut p, "ed@x.org");
    for _ in 0..3 {
        named(&mut p, NamedKey::Tab);
    }
    typed(&mut p, "move lunch to fri");
    named(&mut p, NamedKey::Tab);
    let ask = p.to_smith.take().expect("smith asked");
    assert_eq!(ask.wants, crate::inbox::handoff::Wants::Draft);
    assert!(ask.prompt.contains("to ed@x.org"), "{}", ask.prompt);
    assert!(ask.prompt.contains("move lunch to fri"), "{}", ask.prompt);
    assert!(ask.shown.contains("draft a mail to"), "{}", ask.shown);
    // Smith answers with nothing open but the list: it lands in the box.
    p.waiting = Some(crate::inbox::handoff::Waiting {
        since: 0,
        at_ms: 0,
        wants: crate::inbox::handoff::Wants::Draft,
        thread: ask.thread,
    });
    p.from_smith(Some(vec![
        "<<<DRAFT\nHi Ed, can we do Friday?\nDRAFT>>>".into()
    ]));
    let c = p.compose.as_ref().unwrap();
    assert_eq!(c.body.text, "Hi Ed, can we do Friday?", "notes replaced");
}
