use std::sync::mpsc;

use base64::Engine;
use winit::keyboard::{Key, NamedKey};

use crate::inbox::api::Mail;
use crate::inbox::body::Part;
use crate::inbox::compose::{Kind, On};
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

/// A pane reading Ann's mail to me and Bo, Cy copied, a file with it.
pub(crate) fn reading() -> (InboxPane, mpsc::Receiver<Ask>) {
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
        id: "m1".into(),
        from: "Ann Lee <ann@x.org>".into(),
        date: "Tue, 21 Oct 2026 10:00:00 -0400".into(),
        text: "Friday?".into(),
        attached: vec![Part {
            name: "menu.pdf".into(),
            mime: "application/pdf".into(),
            att: Some("a1".into()),
            size: 2048,
            ..Part::default()
        }],
        ..Mail::default()
    };
    mail.reply.subject = "Lunch".into();
    mail.reply.message_id = "<1@x>".into();
    mail.reply.to = "me@y.org, bo@x.org".into();
    mail.reply.cc = "cy@x.org".into();
    p.reading = Some(Reading {
        mails: Some(vec![mail]),
        ..Reading::new(thread)
    });
    (p, asks)
}

pub(crate) fn press(p: &mut InboxPane, k: NamedKey) {
    p.press(&Key::Named(k), 80, 30);
}

pub(crate) fn typed(p: &mut InboxPane, s: &str) {
    for c in s.chars() {
        p.press(&Key::Character(c.to_string().into()), 80, 30);
    }
}

pub(crate) fn on(p: &InboxPane) -> On {
    p.compose.as_ref().expect("the box").on
}

fn sent(asks: &mpsc::Receiver<Ask>) -> String {
    let raw = asks.try_iter().find_map(|a| match a {
        Ask::Send { raw, .. } => Some(raw),
        _ => None,
    });
    let raw = raw.expect("sent");
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn up_reaches_the_cc_line_and_what_is_typed_there_is_copied() {
    let (mut p, asks) = reading();
    typed(&mut p, "r");
    assert_eq!(on(&p), On::Body, "a reply starts in the words");
    typed(&mut p, "Yes!");
    press(&mut p, NamedKey::ArrowUp);
    assert_eq!(on(&p), On::Cc);
    typed(&mut p, ", dee@x.org");
    let cells = p.cells(80, 30);
    let row = |r: u16| -> String {
        let mut c: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
        c.sort_by_key(|c| c.col);
        c.iter().map(|c| c.c).collect()
    };
    let rows: Vec<String> = (0..30).map(row).collect();
    assert!(
        rows.iter()
            .any(|r| r.contains("to") && r.contains("Ann Lee <ann@x.org>, bo@x.org")),
        "{rows:#?}"
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("cc") && r.contains("cy@x.org, dee@x.org")),
        "{rows:#?}"
    );
    press(&mut p, NamedKey::Enter);
    assert_eq!(on(&p), On::Body, "Enter on a line goes on to the words");
    p.send_reply();
    let m = sent(&asks);
    assert!(
        m.contains("To: Ann Lee <ann@x.org>, bo@x.org\r\nCc: cy@x.org, dee@x.org\r\n"),
        "{m}"
    );
}

#[test]
fn a_reply_copies_everyone_and_a_bad_address_is_refused_where_it_is() {
    let (mut p, asks) = reading();
    typed(&mut p, "r");
    let c = p.compose.as_ref().expect("the box");
    assert_eq!(c.kind, Kind::Reply, "every reply is to everyone");
    assert_eq!(
        (c.to.text.as_str(), c.cc.text.as_str()),
        ("Ann Lee <ann@x.org>, bo@x.org", "cy@x.org")
    );
    typed(&mut p, "See you.");
    press(&mut p, NamedKey::ArrowUp);
    typed(&mut p, ", bob");
    press(&mut p, NamedKey::Tab);
    assert_eq!(on(&p), On::Body, "Tab on a line goes on");
    p.send_reply();
    assert_eq!(p.note, "\u{201c}bob\u{201d} is not an address");
    assert_eq!(on(&p), On::Cc, "the caret goes to it");
    assert!(asks.try_iter().all(|a| !matches!(a, Ask::Send { .. })));
    press(&mut p, NamedKey::End);
    for _ in 0..", bob".len() {
        press(&mut p, NamedKey::Backspace);
    }
    p.send_reply();
    let m = sent(&asks);
    assert!(
        m.contains("To: Ann Lee <ann@x.org>, bo@x.org\r\nCc: cy@x.org\r\n"),
        "{m}"
    );
}

#[test]
fn esc_closes_an_untouched_box_and_asks_twice_once_a_line_changed() {
    let (mut p, _) = reading();
    typed(&mut p, "r");
    press(&mut p, NamedKey::Escape);
    assert!(p.compose.is_none(), "nothing changed: closed at once");
    typed(&mut p, "r");
    press(&mut p, NamedKey::ArrowUp);
    typed(&mut p, "x");
    press(&mut p, NamedKey::Escape);
    assert!(p.compose.is_some(), "a changed cc line is words");
    press(&mut p, NamedKey::Escape);
    assert!(p.compose.is_none());
}

#[test]
fn a_click_puts_the_caret_on_the_line_clicked() {
    let (mut p, _) = reading();
    typed(&mut p, "r");
    let h = p.compose_h(80, 30);
    let top = p.cards(80, 30).reply.expect("the box's card").row;
    assert!(h > 0 && p.compose_click(80, 30, top + 2, 9));
    assert_eq!(on(&p), On::Cc);
    assert!(p.compose_click(80, 30, top + 1, 9));
    assert_eq!(on(&p), On::To);
    assert!(p.compose_click(80, 30, top + 4, 9));
    assert_eq!(on(&p), On::Body);
    assert!(
        !p.compose_click(80, 30, top - 1, 9),
        "above the box is the mail"
    );
}

#[test]
fn tab_asks_smith_once_while_it_writes() {
    let (mut p, _) = reading();
    typed(&mut p, "r");
    press(&mut p, NamedKey::Tab);
    assert!(
        p.to_smith
            .take()
            .is_some_and(|s| s.wants == crate::inbox::handoff::Wants::Draft),
        "asked"
    );
    p.waiting = Some(crate::inbox::handoff::Waiting {
        since: 0,
        at_ms: 0,
        wants: crate::inbox::handoff::Wants::Draft,
    });
    press(&mut p, NamedKey::Tab);
    assert!(p.to_smith.is_none(), "not twice");
    assert!(p.note.contains("still writing"), "{}", p.note);
}
