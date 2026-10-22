use base64::Engine;

use super::{calendar, mail, Answer};
use crate::inbox::ics::tests::{cal, GOOGLE};
use crate::inbox::ics::{parse, Invite};

fn inv() -> Invite {
    parse(&cal("METHOD:REQUEST\r\n", GOOGLE)).unwrap()
}

const STAMP: &str = "20261022T120000Z";

#[test]
fn a_reply_names_the_event_its_organizer_and_you_with_your_answer() {
    let c = calendar(&inv(), "Me@X.org", Answer::Maybe, STAMP).unwrap();
    for want in [
        "METHOD:REPLY\r\n",
        "UID:abc@google.com\r\n",
        "SEQUENCE:1\r\n",
        "DTSTAMP:20261022T120000Z\r\n",
        "ORGANIZER:mailto:ann@x.org\r\n",
        "ATTENDEE;PARTSTAT=TENTATIVE:mailto:me@x.org\r\n",
        "SUMMARY:Q4 planning\r\n",
    ] {
        assert!(c.contains(want), "{want} missing from {c}");
    }
    assert!(c.starts_with("BEGIN:VCALENDAR\r\n") && c.ends_with("END:VCALENDAR\r\n"));
    let other = calendar(&inv(), "alias@x.org", Answer::Yes, STAMP).unwrap();
    assert!(
        other.contains("ATTENDEE;PARTSTAT=ACCEPTED:mailto:alias@x.org"),
        "as your own address"
    );
}

#[test]
fn nothing_in_the_invite_breaks_out_of_its_line() {
    let mut i = inv();
    i.summary = "Q4\r\nATTENDEE;PARTSTAT=ACCEPTED:mailto:x@evil.org".into();
    i.uid = "abc\nMETHOD:CANCEL".into();
    let c = calendar(&i, "me@x.org", Answer::No, STAMP).unwrap();
    // The words stay on their own line: no property of theirs is made.
    let starts = |name: &str| c.lines().filter(|l| l.starts_with(name)).count();
    assert_eq!((starts("ATTENDEE"), starts("METHOD")), (1, 1), "{c}");
}

#[test]
fn the_mail_goes_to_the_organizer_with_the_calendar_beside_its_words() {
    let (raw, to) = mail(&inv(), "me@x.org", Answer::Yes, STAMP).unwrap();
    assert_eq!(to, "ann@x.org");
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .unwrap();
    let m = String::from_utf8(bytes).unwrap();
    for want in [
        "To: ann@x.org",
        "Subject: Accepted: Q4 planning",
        "multipart/alternative",
        "text/plain; charset=UTF-8",
        "text/calendar; method=REPLY; charset=UTF-8",
    ] {
        assert!(m.contains(want), "{want} missing from {m}");
    }
    let mut none = inv();
    none.organizer = None;
    assert!(
        mail(&none, "me@x.org", Answer::Yes, STAMP).is_none(),
        "no one to tell"
    );
}
