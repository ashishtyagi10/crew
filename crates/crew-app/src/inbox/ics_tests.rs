use super::{parse, When};
use chrono::{NaiveDate, NaiveDateTime};

pub(crate) fn at(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S").unwrap()
}

/// Wrap event lines in a calendar, with `head` (a METHOD, zones) before.
pub(crate) fn cal(head: &str, event: &str) -> String {
    format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{head}BEGIN:VEVENT\r\n{event}END:VEVENT\r\nEND:VCALENDAR\r\n")
}

pub(crate) const GOOGLE: &str = "DTSTART;TZID=America/New_York:20261030T100000\r\n\
DTEND;TZID=America/New_York:20261030T104500\r\nUID:abc@google.com\r\nSEQUENCE:1\r\n\
SUMMARY:Q4 planning\r\nORGANIZER;CN=Ann Lee:mailto:Ann@X.org\r\n\
ATTENDEE;CN=Bo Park;PARTSTAT=ACCEPTED:mailto:bo@y.org\r\n\
ATTENDEE;CN=me;PARTSTAT=NEEDS-ACTION:mailto:me@x.org\r\nLOCATION:Room 4\\, east\r\n\
X-GOOGLE-CONFERENCE:https://meet.google.com/abc-defg-hij\r\nBEGIN:VALARM\r\n\
DESCRIPTION:This is an event reminder\r\nTRIGGER:-P0DT0H10M0S\r\nEND:VALARM\r\n\
DESCRIPTION:Agenda\r\n";

#[test]
fn a_google_invite_reads_whole_and_its_alarm_is_not_the_event() {
    let inv = parse(&cal("METHOD:REQUEST\r\n", GOOGLE)).unwrap();
    assert_eq!(
        (inv.method.as_str(), inv.uid.as_str(), inv.sequence),
        ("REQUEST", "abc@google.com", 1)
    );
    let ny = Some("America/New_York".to_string());
    assert_eq!(
        inv.start,
        Some(When::Local(at("20261030T100000"), ny.clone()))
    );
    assert_eq!(inv.end, Some(When::Local(at("20261030T104500"), ny)));
    assert_eq!(
        (inv.summary.as_str(), inv.location.as_str()),
        ("Q4 planning", "Room 4, east")
    );
    assert_eq!(inv.description, "Agenda", "the alarm's words are its own");
    assert_eq!(
        inv.link.as_deref(),
        Some("https://meet.google.com/abc-defg-hij")
    );
    let org = inv.organizer.unwrap();
    assert_eq!(
        (org.name.as_str(), org.email.as_str()),
        ("Ann Lee", "ann@x.org")
    );
    let answers: Vec<&str> = inv.attendees.iter().map(|a| a.answer.as_str()).collect();
    assert_eq!(answers, ["ACCEPTED", "NEEDS-ACTION"]);
    assert!(!inv.cancelled);
}

#[test]
fn a_duration_gives_the_end_and_a_bare_date_is_a_whole_day() {
    let day = parse(&cal("", "DTSTART;VALUE=DATE:20261126\r\nDURATION:P1D\r\n")).unwrap();
    let d = |m, d| When::Day(NaiveDate::from_ymd_opt(2026, m, d).unwrap());
    assert_eq!((day.start, day.end), (Some(d(11, 26)), Some(d(11, 27))));
    let utc = parse(&cal("", "DTSTART:20261104T150000Z\r\nDURATION:PT1H30M\r\n")).unwrap();
    assert_eq!(utc.end, Some(When::Utc(at("20261104T163000"))));
}

#[test]
fn the_series_is_read_not_one_moved_occurrence_of_it() {
    let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nRECURRENCE-ID:20261028T093000Z\r\n\
SUMMARY:Standup (moved)\r\nDTSTART:20261028T110000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\n\
SUMMARY:Standup\r\nDTSTART:20261026T133000Z\r\nRRULE:FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR\r\n\
END:VEVENT\r\nEND:VCALENDAR\r\n";
    let inv = parse(ics).unwrap();
    assert_eq!(inv.summary, "Standup");
    assert_eq!(
        inv.rrule.as_deref(),
        Some("FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR")
    );
}

#[test]
fn a_cancellation_says_so_and_a_file_with_no_start_is_no_invite() {
    let ev = "DTSTART:20261102T190000Z\r\n";
    assert!(parse(&cal("METHOD:CANCEL\r\n", ev)).unwrap().cancelled);
    let status = format!("{ev}STATUS:CANCELLED\r\n");
    assert!(
        parse(&cal("METHOD:REQUEST\r\n", &status))
            .unwrap()
            .cancelled
    );
    assert!(parse(&cal("", "SUMMARY:no time\r\n")).is_none());
}

#[test]
fn a_meeting_link_is_found_in_the_words_when_google_names_none() {
    let ev = "DTSTART:20261104T150000Z\r\nLOCATION:Zoom\r\n\
DESCRIPTION:See you!\\nJoin: https://us02web.zoom.us/j/8812345678?pwd=abc.\\n\r\n";
    let inv = parse(&cal("", ev)).unwrap();
    assert_eq!(
        inv.link.as_deref(),
        Some("https://us02web.zoom.us/j/8812345678?pwd=abc")
    );
    let none = parse(&cal(
        "",
        "DTSTART:20261104T150000Z\r\nDESCRIPTION:https://x.org/a\r\n",
    ))
    .unwrap();
    assert_eq!(none.link, None, "only a meeting's link");
}

/// Only an invite's own methods are invites: a proposed new time, its
/// refusal and the rest say what they are, and offer no keys.
#[test]
fn a_proposed_time_is_said_and_is_no_invite() {
    let me = "ATTENDEE;CN=me;PARTSTAT=NEEDS-ACTION:mailto:me@x.org\r\n";
    let line = |method: &str| {
        let ics = cal(&format!("METHOD:{method}\r\n"), &GOOGLE.replace(me, ""));
        parse(&ics).unwrap().reply_line()
    };
    assert_eq!(
        line("COUNTER").as_deref(),
        Some("Bo Park proposed a new time")
    );
    assert_eq!(
        line("DECLINECOUNTER").as_deref(),
        Some("your proposed time was turned down")
    );
    assert_eq!(
        line("REFRESH").as_deref(),
        Some("a calendar refresh from Bo Park")
    );
    assert_eq!(
        line("REPLY").as_deref(),
        Some("Bo Park accepted your invite")
    );
    for invite in ["REQUEST", "PUBLISH", "CANCEL"] {
        assert_eq!(line(invite), None, "{invite}");
    }
}
