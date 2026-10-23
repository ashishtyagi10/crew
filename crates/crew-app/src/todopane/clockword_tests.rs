use crate::todopane::duedate::{find, strip};
use chrono::{NaiveDate, NaiveDateTime};

/// Tuesday 2026-10-20, 08:00.
fn tue() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, 20)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap()
}

fn read(input: &str) -> (String, String) {
    let h = find(input, tue()).expect("a due");
    (
        h.due.format("%Y-%m-%d %H:%M").to_string(),
        strip(input, h.start, h.end),
    )
}

#[test]
fn a_clock_and_its_am_or_pm_a_word_apart_are_one_time() {
    assert_eq!(
        read("call mom at 5 pm"),
        ("2026-10-20 17:00".into(), "call mom".into())
    );
    assert_eq!(
        read("standup tomorrow 9 am"),
        ("2026-10-21 09:00".into(), "standup".into())
    );
    assert_eq!(
        read("dentist fri at 4:30 pm"),
        ("2026-10-23 16:30".into(), "dentist".into())
    );
}

/// A word that is no clock keeps its am, and nothing becomes a due.
#[test]
fn only_a_clock_takes_the_am_or_pm() {
    assert!(find("i am here", tue()).is_none());
    assert!(find("room 1234 pm review", tue()).is_none());
}
