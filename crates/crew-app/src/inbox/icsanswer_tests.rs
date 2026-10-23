use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use crate::inbox::api::Mail;
use crate::inbox::ics::reply::Answer;
use crate::inbox::ics::tests::{cal, GOOGLE};
use crate::inbox::worker::{Ask, Got, Worker};
use crate::inbox::{wire::Thread, InboxPane, Reading, Stage};

const UID: &str = "abc@google.com";

/// A signed-in pane whose worker is a channel this test reads, open on one
/// mail carrying `ics`.
fn pane(ics: String) -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.email = "me@x.org".into();
    let m = Mail {
        calendar: Some(ics),
        ..Default::default()
    };
    p.reading = Some(Reading {
        mails: Some(vec![m]),
        ..Reading::new(Thread::default())
    });
    (p, asks)
}

fn invited() -> (InboxPane, mpsc::Receiver<Ask>) {
    pane(cal("METHOD:REQUEST\r\n", GOOGLE))
}

fn press(p: &mut InboxPane, k: &str) {
    let key = match k {
        "esc" => Key::Named(NamedKey::Escape),
        c => Key::Character(c.into()),
    };
    p.press_key(&key, 100, 40);
}

/// The card's lines, as text.
fn card(p: &InboxPane) -> Vec<String> {
    let laid = p.reading.as_ref().unwrap().laid(80);
    let text = laid
        .lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect::<String>());
    text.filter(|l| l.starts_with(" \u{258e} ")).collect()
}

#[test]
fn y_names_who_will_be_told_and_only_a_second_key_sends() {
    let (mut p, asks) = invited();
    press(&mut p, "y");
    assert!(
        p.answering() && p.note.starts_with("Ann Lee will be told"),
        "{}",
        p.note
    );
    assert!(
        card(&p).iter().any(|l| l.contains("answer Ann Lee: y yes")),
        "{:?}",
        card(&p)
    );
    assert!(asks.try_recv().is_err(), "nothing went on the first press");
    press(&mut p, "y");
    assert!(!p.answering());
    let sent = asks.try_recv().unwrap();
    assert!(matches!(sent, Ask::Answer { answer: Answer::Yes, ref me, .. } if me == "me@x.org"));
}

#[test]
fn while_answering_m_is_maybe_not_mark_read_and_esc_sends_nothing() {
    let (mut p, asks) = invited();
    press(&mut p, "y");
    press(&mut p, "j");
    assert!(p.answering(), "any other key waits");
    press(&mut p, "m");
    assert!(matches!(
        asks.try_recv().unwrap(),
        Ask::Answer {
            answer: Answer::Maybe,
            ..
        }
    ));
    assert!(asks.try_recv().is_err(), "no mark-read with it");
    press(&mut p, "y");
    press(&mut p, "esc");
    assert!(!p.answering() && asks.try_recv().is_err() && p.note.is_empty());
}

#[test]
fn a_cancelled_invite_has_nothing_to_answer() {
    let (mut p, _) = pane(cal("METHOD:CANCEL\r\n", GOOGLE));
    press(&mut p, "y");
    assert!(
        !p.answering() && p.note.contains("nothing to answer"),
        "{}",
        p.note
    );
}

#[test]
fn c_goes_to_google_calendar_once_it_has_answered_for_the_invite() {
    let (mut p, asks) = invited();
    p.take_cal(Got::OnCalendar(UID.into(), None));
    press(&mut p, "c");
    assert!(matches!(asks.try_recv().unwrap(), Ask::AddEvent(_)));
    assert!(
        p.note.starts_with("adding it to your Google Calendar"),
        "{}",
        p.note
    );
    p.take_cal(Got::OnCalendar(UID.into(), Some("accepted".into())));
    press(&mut p, "c");
    assert_eq!(p.note, "already on your Google Calendar");
    assert!(asks.try_recv().is_err());
}

#[test]
fn the_card_says_where_the_invite_stands() {
    let (mut p, _) = invited();
    assert!(card(&p)
        .iter()
        .any(|l| l.ends_with("c adds it to your calendar \u{b7} y answers it")));
    p.take_cal(Got::OnCalendar(UID.into(), Some("needsAction".into())));
    let on = "\u{2713} on your Google Calendar \u{b7} you have not answered";
    assert!(card(&p).iter().any(|l| l.ends_with(on)), "{:?}", card(&p));
    let line = "you accepted \u{b7} Ann Lee was told".to_string();
    p.take_cal(Got::Answered(UID.into(), "accepted".into(), line.clone()));
    assert!(card(&p)
        .iter()
        .any(|l| l.ends_with("Google Calendar \u{b7} you accepted")));
    assert_eq!(p.note, line);
}

/// An "Accepted: …" mail carries an attendee's answer (`METHOD:REPLY`), not
/// an invite: its card offered `c` and `y`, and `y` then `y` sent a REPLY to
/// yourself. Now the card says who answered, with no keys, and the keys say
/// why they do nothing.
#[test]
fn an_answer_to_your_invite_is_not_an_invite() {
    let me = "ATTENDEE;CN=me;PARTSTAT=NEEDS-ACTION:mailto:me@x.org\r\n";
    let (mut p, asks) = pane(cal("METHOD:REPLY\r\n", &GOOGLE.replace(me, "")));
    let c = card(&p);
    assert!(
        c.iter().any(|l| l.contains("Bo Park accepted your invite")),
        "{c:?}"
    );
    assert!(!c.iter().any(|l| l.contains("y answers it")), "{c:?}");
    for k in ["y", "y", "c"] {
        press(&mut p, k);
        assert!(
            !p.answering() && p.note.contains("an answer"),
            "{k}: {}",
            p.note
        );
    }
    assert!(asks.try_recv().is_err(), "nothing asked of Gmail or Google");
}
