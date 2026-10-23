use crate::todopane::duedate::{find, strip};
use chrono::{NaiveDate, NaiveDateTime};

/// Tuesday 2026-10-20, 10:00.
fn tue() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, 20)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn read(input: &str) -> (String, String) {
    let h = find(input, tue()).expect("a date");
    (
        h.due.format("%Y-%m-%d %H:%M").to_string(),
        strip(input, h.start, h.end),
    )
}

/// The probe's words: the title loses `next`, and the Friday is next week's.
#[test]
fn next_friday_is_the_week_afters_and_leaves_the_title() {
    assert_eq!(
        read("call bob next friday"),
        ("2026-10-30 09:00".into(), "call bob".into())
    );
    assert_eq!(
        read("call bob this friday"),
        ("2026-10-23 09:00".into(), "call bob".into())
    );
    assert_eq!(
        read("standup next mon 9am"),
        ("2026-10-26 09:00".into(), "standup".into())
    );
    assert_eq!(
        read("pay rent by next tue"),
        ("2026-10-27 09:00".into(), "pay rent".into())
    );
}

/// `next` alone, or before a word that is no day, stays in the title.
#[test]
fn next_without_a_day_is_just_a_word() {
    assert!(find("plan the next step", tue()).is_none());
    assert_eq!(read("next steps friday").1, "next steps");
}
