//! The deadline a mail states, read as of the day it was sent.
use super::{day, deadline};
use chrono::{NaiveDate, NaiveDateTime};

/// Tuesday 2026-10-20, 09:30: the day the mails below were sent.
fn tue() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, 20)
        .unwrap()
        .and_hms_opt(9, 30, 0)
        .unwrap()
}

fn due(text: &str) -> Option<String> {
    deadline(text, tue()).map(|d| d.0.format("%Y-%m-%d").to_string())
}

/// An overdue notice names a date before it was sent: that date, passed, and
/// not next year's (it was 2027-10-15).
#[test]
fn a_month_and_day_already_passed_stays_passed() {
    assert_eq!(
        due("This was due by Oct 15, please send it today.").as_deref(),
        Some("2026-10-15")
    );
    assert_eq!(due("Please pay by Dec 1.").as_deref(), Some("2026-12-01"));
    assert_eq!(due("Renew it by Jan 15.").as_deref(), Some("2027-01-15"));
    assert_eq!(
        due("The report was due by June 1.").as_deref(),
        Some("2026-06-01")
    );
}

/// What says its year, or counts days, means it.
#[test]
fn a_written_year_and_a_count_of_days_are_kept() {
    assert_eq!(due("Due 2027-10-15.").as_deref(), Some("2027-10-15"));
    assert_eq!(due("Due in 300 days.").as_deref(), Some("2027-08-16"));
}

/// "by" and "due" inside a word are not deadlines.
#[test]
fn a_key_inside_a_word_is_not_a_deadline() {
    assert_eq!(due("Meet me in the lobby at 3pm."), None);
    assert_eq!(due("The overdue invoice from Oct 3 is attached."), None);
    assert_eq!(
        due("On standby until Friday.").as_deref(),
        Some("2026-10-23")
    );
}

/// The reason line says the year when it is not this one.
#[test]
fn a_day_in_another_year_says_its_year() {
    let then = NaiveDate::from_ymd_opt(1999, 1, 1)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    assert_eq!(day(then, false), "Fri Jan 1 1999");
    let now = crate::todopane::duedate::now_local();
    assert!(!day(now, false).contains(&now.format("%Y").to_string()));
}

/// "by Friday at 5pm" is Friday at five, said as it was written.
#[test]
fn a_deadline_at_a_time_is_that_day_at_that_time() {
    let d = deadline("Please send the deck by Friday at 5pm.", tue()).expect("a deadline");
    assert_eq!(d.0.format("%Y-%m-%d %H:%M").to_string(), "2026-10-23 17:00");
    assert!(d.1, "a time was said");
    assert_eq!(d.2, "by friday at 5pm");
}

/// A date months ahead is ahead: only an overdue notice, or one weeks back,
/// reads a month and day as passed.
#[test]
fn a_date_months_ahead_is_not_read_as_passed() {
    assert_eq!(
        due("Applications are due by May 15.").as_deref(),
        Some("2027-05-15")
    );
    assert_eq!(
        due("The offer is valid until June 30.").as_deref(),
        Some("2027-06-30")
    );
    assert_eq!(
        due("The report was due by June 1.").as_deref(),
        Some("2026-06-01"),
        "an overdue notice"
    );
}

/// A day after an end-of-day word is that day's end, and "next Friday" keeps
/// its time: "by EOD Friday" was due the Tuesday it was sent, at five.
#[test]
fn an_end_of_day_names_its_day_and_next_keeps_its_time() {
    let read = |s| {
        let d = deadline(s, tue()).unwrap();
        (d.0.format("%Y-%m-%d %H:%M").to_string(), d.1, d.2)
    };
    for (mail, due, said) in [
        (
            "Please send it by EOD Friday.",
            "2026-10-23 17:00",
            "by eod friday",
        ),
        (
            "Numbers by end of day tomorrow?",
            "2026-10-21 17:00",
            "by end of day tomorrow",
        ),
        (
            "Need sign-off by COB on Monday.",
            "2026-10-26 17:00",
            "by cob monday",
        ),
        (
            "Need the slides by EOD please.",
            "2026-10-20 17:00",
            "by eod",
        ),
        (
            "Send the deck by next Friday at 5pm.",
            "2026-10-30 17:00",
            "by next friday at 5pm",
        ),
    ] {
        assert_eq!(read(mail), (due.into(), true, said.into()), "{mail}");
    }
}

/// A key's date comes right after it. "By the way, Friday works for me" set a
/// deadline of Friday, and "Signed by Ann on Oct 15" one quoted as "by on oct
/// 15", words no mail said.
#[test]
fn a_key_brings_its_date_right_after_it() {
    assert_eq!(due("By the way, Friday works for me."), None);
    assert_eq!(due("Signed by Ann on Oct 15."), None);
    assert_eq!(
        due("Please send it by Friday, Oct 23.").as_deref(),
        Some("2026-10-23")
    );
    assert_eq!(
        due("Can you review it by tomorrow morning?").as_deref(),
        Some("2026-10-21")
    );
}

/// A weekday may lead the date it names, and "is" or "date" may stand
/// between a key and its date: after I29, "by Friday, October 30" read a week
/// early (this Friday) and "Due date: Oct 30" read nothing.
#[test]
fn a_key_reads_past_a_weekday_and_an_is() {
    assert_eq!(
        due("Please send it by Friday, October 30.").as_deref(),
        Some("2026-10-30")
    );
    assert_eq!(
        due("The deadline is Friday.").as_deref(),
        Some("2026-10-23")
    );
    assert_eq!(due("Due date: Oct 30.").as_deref(), Some("2026-10-30"));
    assert_eq!(
        due("The due date is October 30.").as_deref(),
        Some("2026-10-30")
    );
    // I29's own cases hold.
    assert_eq!(due("By the way, Friday works for me."), None);
    assert_eq!(due("Signed by Ann on Oct 15."), None);
}
