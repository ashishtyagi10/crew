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
