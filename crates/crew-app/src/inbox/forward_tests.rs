use winit::keyboard::NamedKey;

use super::{subject, Forwarded, Source};
use crate::inbox::api::Mail;
use crate::inbox::body::Part;
use crate::inbox::compose::{Kind, On};
use crate::inbox::composekeys::tests::{on, press, reading, typed};
use crate::inbox::worker::{Ask, Got};

fn part(name: &str, cid: Option<&str>, att: &str) -> Part {
    Part {
        cid: cid.map(str::to_string),
        name: name.into(),
        mime: "image/png".into(),
        att: Some(att.into()),
        size: 10,
        ..Part::default()
    }
}

pub(crate) fn mail() -> Mail {
    let mut m = Mail {
        id: "m1".into(),
        from: "Ann Lee <ann@x.org>".into(),
        date: "Tue, 21 Oct 2026".into(),
        text: "**Hi** there".into(),
        parts: vec![
            part("logo.png", Some("logo@x"), "a1"),
            part("", Some("unused@x"), "a2"),
        ],
        attached: vec![
            part("logo.png", None, "a1"),
            Part {
                mime: "application/pdf".into(),
                ..part("menu.pdf", None, "a3")
            },
        ],
        ..Mail::default()
    };
    m.reply.subject = "Lunch".into();
    m.reply.to = "me@y.org".into();
    m.reply.source = Source {
        html: Some("<html><head><style>.a{color:red}</style></head><body class=\"x\"><p class=\"a\">Hi <img src=\"cid:logo@x\"></p></body></html>".into()),
        plain: None,
    };
    m
}

#[test]
fn a_forward_carries_the_pictures_its_html_draws_and_its_files_once() {
    let f = Forwarded::of(&mail());
    let names = |ps: &[Part]| ps.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&f.inline), ["logo.png"], "only what the HTML names");
    assert_eq!(names(&f.files), ["menu.pdf"], "the logo is not a file too");
    assert_eq!(f.size(), 20);
    assert_eq!(subject("Lunch"), "Fwd: Lunch");
    assert_eq!(subject("FW: Lunch"), "FW: Lunch");
    assert_eq!(subject(" fwd: x "), "fwd: x");
}

#[test]
fn the_upload_root_puts_upload_before_the_path() {
    let root = crate::inbox::upload::root;
    assert_eq!(
        root("https://gmail.googleapis.com/gmail/v1/users/me"),
        "https://gmail.googleapis.com/upload/gmail/v1/users/me"
    );
    assert_eq!(
        root("http://127.0.0.1:9/gmail"),
        "http://127.0.0.1:9/upload/gmail"
    );
}

#[test]
fn f_forwards_the_newest_mail_to_whom_you_say_with_its_file() {
    let (mut p, asks) = reading();
    typed(&mut p, "f");
    let c = p.compose.as_ref().expect("the box");
    assert_eq!(
        (c.kind, c.on),
        (Kind::Forward, On::To),
        "the caret waits on to"
    );
    assert_eq!(c.answer.subject, "Fwd: Lunch");
    p.send_reply();
    assert_eq!(p.note, "who is it to? write an address on the to line");
    typed(&mut p, "Dee <dee@x.org>");
    for _ in 0..2 {
        press(&mut p, NamedKey::Tab);
    }
    assert_eq!(on(&p), On::Body, "through cc");
    press(&mut p, NamedKey::Tab);
    assert_eq!(on(&p), On::Files, "no smith in a forward: Tab goes on");
    press(&mut p, NamedKey::Tab);
    assert_eq!(on(&p), On::To, "and round");
    let cells = p.cells(80, 30);
    let all: String = cells.iter().map(|c| c.c).collect();
    assert!(all.contains("menu.pdf"), "the box says what it carries");
    p.send_reply();
    let fwd = asks.try_iter().find_map(|a| match a {
        Ask::Mail(o) => Some(o),
        _ => None,
    });
    let o = fwd.expect("a note is not needed to forward");
    assert_eq!(o.head, ["To: Dee <dee@x.org>", "Subject: Fwd: Lunch"]);
    assert_eq!(o.thread_id, None, "a thread of its own");
    assert_eq!((o.words.as_str(), o.to.as_str()), ("", "Dee <dee@x.org>"));
    let of = o.forward.as_ref().expect("the mail it carries");
    assert_eq!((of.msg.as_str(), of.files.len()), ("m1", 1));
    p.take(Got::Forwarded(o.to.clone()));
    assert!(p.compose.is_none());
    assert_eq!(p.note, "forwarded to Dee");
}

#[test]
fn a_forward_past_gmails_limit_is_said_not_sent() {
    let (mut p, asks) = reading();
    if let Some(m) = p.reading.as_mut().and_then(|r| r.mails.as_mut()) {
        m[0].attached[0].size = 30 * 1024 * 1024;
    }
    typed(&mut p, "f");
    typed(&mut p, "dee@x.org");
    p.send_reply();
    assert!(p.note.contains("30 MB, more than the 25 MB"), "{}", p.note);
    assert!(asks.try_iter().all(|a| !matches!(a, Ask::Mail(_))));
    assert!(!p.compose.as_ref().unwrap().sending);
}
