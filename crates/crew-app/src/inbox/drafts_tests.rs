//! Drafts: saved, updated, sent as the draft, and put back in the box.
use std::sync::mpsc;

use serde_json::json;
use winit::keyboard::{Key, NamedKey};

use super::{ids, save, Draft};
use crate::inbox::api::{Gmail, Mail};
use crate::inbox::compose::{Kind, On};
use crate::inbox::forwardmail::tests::serve;
use crate::inbox::grant::{Client, Grant};
use crate::inbox::outgoing::{self, Outgoing};
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Got, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

fn gmail() -> (Gmail, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
    let (base, log) = serve();
    let grant = Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
        scope: String::new(),
    };
    let g = Gmail::at(grant, &format!("{base}/token"), &format!("{base}/gmail")).unwrap();
    (g, log)
}

fn mail(thread: Option<&str>, draft: Option<&str>) -> Outgoing {
    Outgoing {
        head: vec!["To: ann@x.org".into(), "Subject: Plans".into()],
        thread_id: thread.map(Into::into),
        words: "half written".into(),
        forward: None,
        files: vec![],
        to: "ann@x.org".into(),
        draft: draft.map(Into::into),
    }
}

#[test]
fn a_draft_is_created_then_updated_in_place_and_sent_as_itself() {
    let (mut g, log) = gmail();
    assert_eq!(save(&mut g, &mail(None, None)).as_deref(), Ok("sent"));
    save(&mut g, &mail(Some("t1"), Some("r9"))).expect("updated");
    outgoing::send(&mut g, &mail(Some("t1"), Some("r9"))).expect("sent");
    let log = log.lock().unwrap().clone();
    let made = log
        .iter()
        .find(|r| r.starts_with("POST /upload/gmail/drafts?uploadType=media "))
        .expect("a new draft on no thread: the bare message");
    assert!(made.contains("Subject: Plans\r\n"), "{made}");
    let put = log
        .iter()
        .filter(|r| r.starts_with("PUT /upload/gmail/drafts/r9?uploadType=multipart "))
        .collect::<Vec<_>>();
    assert_eq!(
        put.len(),
        2,
        "saved, then brought up to date before it went"
    );
    assert!(
        put[0].contains(r#"{"id":"r9","message":{"threadId":"t1"}}"#),
        "{}",
        put[0]
    );
    let sent = log
        .iter()
        .find(|r| r.starts_with("POST /gmail/drafts/send "))
        .expect("sent as the draft");
    assert!(sent.contains(r#"{"id":"r9"}"#), "{sent}");
    assert!(
        !log.iter().any(|r| r.contains("/messages/send")),
        "not sent twice"
    );
}

#[test]
fn drafts_list_names_each_draft_by_its_message() {
    let v = json!({"drafts": [{"id": "r1", "message": {"id": "m1", "threadId": "t"}}]});
    assert_eq!(ids(&v), vec![("r1".to_string(), "m1".to_string())]);
}

/// A pane on a thread that has landed, and what it asks of Gmail.
fn pane(mails: Vec<Mail>) -> (InboxPane, mpsc::Receiver<Ask>, mpsc::Sender<Got>) {
    let (tx, asks) = mpsc::channel();
    let (answer, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    let thread = Thread {
        id: "t1".into(),
        subject: "Lunch".into(),
        ..Thread::default()
    };
    p.rows = vec![thread.clone()];
    p.email = "me@y.org".into();
    p.reading = Some(Reading::new(thread));
    answer.send(Got::Opened("t1".into(), mails)).unwrap();
    (p, asks, answer)
}

fn draft_mail(id: &str, text: &str) -> Mail {
    let mut m = Mail {
        id: "m2".into(),
        from: "me@y.org".into(),
        text: text.into(),
        draft: Some(Box::new(Draft {
            id: id.into(),
            files: vec![],
        })),
        ..Mail::default()
    };
    m.reply.to = "Ann Lee <ann@x.org>".into();
    m.reply.subject = "Re: Lunch".into();
    m.reply.references = "<a@x> <b@x>".into();
    m
}

fn theirs() -> Mail {
    Mail {
        id: "m1".into(),
        from: "Ann Lee <ann@x.org>".into(),
        ..Mail::default()
    }
}

#[test]
fn a_thread_with_a_reply_drafted_opens_with_the_draft_in_the_box() {
    let (mut p, asks, _a) = pane(vec![theirs(), draft_mail("r9", "Sure, noon.\n")]);
    p.poll();
    let c = p.compose.as_ref().expect("the draft, in the box");
    assert_eq!((c.kind, c.on), (Kind::Reply, On::Body));
    assert_eq!(c.body.text, "Sure, noon.");
    assert_eq!(c.draft.as_deref(), Some("r9"));
    assert_eq!(
        (c.answer.thread_id.as_str(), c.answer.in_reply_to.as_str()),
        ("t1", "<b@x>")
    );
    let shown = p.reading.as_ref().unwrap().mails.as_ref().unwrap();
    assert_eq!(shown.len(), 1, "the draft is not drawn as a mail");
    // Untouched: Esc closes it at once, the draft left in Gmail.
    p.press(&Key::Named(NamedKey::Escape), 80, 30);
    assert!(p.compose.is_none());
    assert!(asks.try_iter().all(|a| !matches!(a, Ask::SaveDraft(_))));
}

#[test]
fn a_draft_alone_in_its_thread_is_a_new_mail_under_the_list() {
    let mut m = draft_mail("r3", "notes");
    m.reply.subject = "Plans".into();
    m.reply.references.clear();
    let (mut p, _asks, _a) = pane(vec![m]);
    p.poll();
    let c = p.compose.as_ref().expect("the box");
    assert_eq!((c.kind, c.subject.text.as_str()), (Kind::New, "Plans"));
    assert!(p.reading.is_none(), "back on the list");
}

#[test]
fn cmd_s_saves_the_box_and_it_closes_when_gmail_has_it() {
    let (mut p, asks, answer) = pane(vec![theirs()]);
    p.poll();
    p.open_new();
    assert!(p.save_draft());
    assert_eq!(p.note, "nothing to save yet");
    p.compose.as_mut().unwrap().body.put("later", false);
    assert!(p.save_draft());
    let o = asks.try_iter().find_map(|a| match a {
        Ask::SaveDraft(o) => Some(o),
        _ => None,
    });
    assert_eq!(o.expect("saved").words, "later");
    assert!(p.compose.as_ref().unwrap().sending, "held while it saves");
    answer.send(Got::DraftSaved(Ok("r1".into()))).unwrap();
    p.poll();
    assert!(p.compose.is_none());
    assert!(p.note.contains("saved to Gmail drafts"), "{}", p.note);
}

#[test]
fn a_reopened_draft_sends_as_the_draft() {
    let (mut p, asks, _a) = pane(vec![theirs(), draft_mail("r9", "Sure.")]);
    p.poll();
    p.send_reply();
    let o = asks.try_iter().find_map(|a| match a {
        Ask::Mail(o) => Some(o),
        _ => None,
    });
    assert_eq!(o.expect("built").draft.as_deref(), Some("r9"));
}

#[test]
fn shift_d_in_the_list_searches_the_drafts() {
    let (mut p, _asks, _a) = pane(vec![]);
    p.reading = None;
    p.press(&Key::Character("D".into()), 80, 30);
    assert_eq!(p.query, "in:drafts");
}
