//! An invite Google Calendar does not have (forwarded, sent to an alias, or
//! from an organizer Calendar does not take invites from) is answered by
//! mail. That puts nothing on your calendar, so the card must not say so,
//! and `c` still adds it.
use std::sync::mpsc;
use winit::keyboard::Key;

use crate::inbox::api::Mail;
use crate::inbox::ics::tests::{cal, GOOGLE};
use crate::inbox::worker::{Ask, Got, Worker};
use crate::inbox::{wire::Thread, InboxPane, Reading, Stage};

const UID: &str = "abc@google.com";

/// The invite card's lines.
fn card(p: &InboxPane) -> Vec<String> {
    let laid = p.reading.as_ref().unwrap().laid(80);
    let text = (laid.lines.iter()).map(|l| l.iter().map(|c| c.c).collect::<String>());
    text.filter(|l| l.starts_with(" \u{258e} ")).collect()
}

#[test]
fn an_answer_by_mail_puts_nothing_on_google_calendar() {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.email = "me@x.org".into();
    let m = Mail {
        calendar: Some(cal("METHOD:REQUEST\r\n", GOOGLE)),
        ..Default::default()
    };
    p.reading = Some(Reading {
        mails: Some(vec![m]),
        ..Reading::new(Thread::default())
    });
    p.take_cal(Got::OnCalendar(UID.into(), None));
    let line = "you accepted \u{b7} Ann Lee was told".to_string();
    p.take_cal(Got::Answered(UID.into(), "accepted".into(), line));
    let lines = card(&p);
    assert!(
        !lines.iter().any(|l| l.contains("on your Google Calendar")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.ends_with("you accepted")),
        "{lines:?}"
    );
    p.press_key(&Key::Character("c".into()), 100, 40);
    assert!(
        matches!(asks.try_recv(), Ok(Ask::AddEvent(_))),
        "{}",
        p.note
    );
}

/// Answered by mail, then added with `c`: Google's copy carries the
/// invite's "needs action" for you, not what you mailed, and the card said
/// "on your Google Calendar · you have not answered".
#[test]
fn an_answer_by_mail_outlives_the_add() {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.email = "me@x.org".into();
    let m = Mail {
        calendar: Some(cal("METHOD:REQUEST\r\n", GOOGLE)),
        ..Default::default()
    };
    p.reading = Some(Reading {
        mails: Some(vec![m]),
        ..Reading::new(Thread::default())
    });
    p.take_cal(Got::OnCalendar(UID.into(), None));
    let line = "you accepted \u{b7} Ann Lee was told".to_string();
    p.take_cal(Got::Answered(UID.into(), "accepted".into(), line));
    p.press_key(&Key::Character("c".into()), 100, 40);
    assert!(
        matches!(asks.try_recv(), Ok(Ask::AddEvent(_))),
        "{}",
        p.note
    );
    // The worker's answer: the add, then the event Google now has.
    p.take_cal(Got::Calendar(
        "added to your Google Calendar: Offsite".into(),
    ));
    p.take_cal(Got::OnCalendar(UID.into(), Some("needsAction".into())));
    let lines = card(&p);
    let on = lines.iter().find(|l| l.contains("on your Google Calendar"));
    assert!(on.is_some_and(|l| l.ends_with("you accepted")), "{lines:?}");
}
