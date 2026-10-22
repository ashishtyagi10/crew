use super::file_name;
use crate::inbox::api::Mail;
use crate::inbox::ics::tests::{cal, GOOGLE};
use crate::inbox::{wire::Thread, InboxPane, Reading, Stage};

/// A signed-out pane (nothing reaches Google) reading one mail carrying `ics`.
fn reading(ics: Option<String>) -> InboxPane {
    let mut p = InboxPane::with(Stage::Setup { found: None });
    let m = Mail {
        calendar: ics,
        ..Default::default()
    };
    p.reading = Some(Reading {
        mails: Some(vec![m]),
        ..Reading::new(Thread::default())
    });
    p
}

fn opened() -> Vec<String> {
    crate::linkclick::OPENED.with(|o| std::mem::take(&mut *o.borrow_mut()))
}

#[test]
fn c_writes_the_invite_as_it_came_and_opens_it_in_the_calendar_app() {
    opened();
    let ics = cal("METHOD:REQUEST\r\n", GOOGLE);
    let note = reading(Some(ics.clone())).add_to_calendar();
    assert!(note.starts_with("opened in "), "{note}");
    let paths = opened();
    assert_eq!(paths.len(), 1);
    assert!(
        paths[0].ends_with("crew-invite-abcgooglecom.ics"),
        "{paths:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&paths[0]).unwrap(),
        ics,
        "byte for byte"
    );
}

#[test]
fn c_adds_nothing_without_an_invite_for_a_cancelled_one_or_a_zone_crew_cannot_place() {
    opened();
    let said = |ics: Option<String>| reading(ics).add_to_calendar();
    assert_eq!(said(None), "no invite in this thread");
    let gone = cal("METHOD:CANCEL\r\n", "DTSTART:20261102T190000Z\r\n");
    assert!(said(Some(gone)).contains("cancelled"));
    let mars = cal("", "DTSTART;TZID=Mars/Olympus_Mons:20261109T100000\r\n");
    assert!(said(Some(mars)).contains("cannot place the zone Mars/Olympus_Mons"));
    assert!(opened().is_empty(), "nothing was opened");
}

#[test]
fn the_file_is_named_for_the_invite_and_nothing_else() {
    assert_eq!(file_name("abc@google.com"), "crew-invite-abcgooglecom.ics");
    assert_eq!(file_name("../../etc/passwd"), "crew-invite-etcpasswd.ics");
    assert_eq!(file_name(""), "crew-invite.ics");
}
