//! The preposition in front of a date leaves the title with the date — and
//! only when it is actually in front of one. `now` is fixed at Wednesday
//! 2026-08-12 12:00, the same fixture the grammar's own table uses.
use super::super::duedate::{find, strip};
use chrono::{NaiveDate, NaiveDateTime};

fn now() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 12)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

/// The title a fragment leaves behind, and the fragment itself — what the
/// composer tints is exactly what the row will not say.
fn split(input: &str) -> (String, String) {
    let hit = find(input, now()).unwrap_or_else(|| panic!("no parse: {input}"));
    let frag: String = input
        .chars()
        .skip(hit.start)
        .take(hit.end - hit.start)
        .collect();
    (strip(input, hit.start, hit.end), frag)
}

#[test]
fn a_lead_in_word_goes_with_the_date_it_introduces() {
    let cases: &[(&str, &str, &str)] = &[
        ("pay rent due friday", "pay rent", "due friday"),
        ("pay rent due tomorrow", "pay rent", "due tomorrow"),
        ("ship build by fri 5pm", "ship build", "by fri 5pm"),
        ("dentist on aug 15", "dentist", "on aug 15"),
        ("standup at 9am", "standup", "at 9am"),
        (
            "file taxes before 2026-12-01",
            "file taxes",
            "before 2026-12-01",
        ),
        // A run of them, and the capitals people actually type.
        ("pay rent due by friday", "pay rent", "due by friday"),
        ("pay rent Due Friday", "pay rent", "Due Friday"),
        // The date still rides its own tail: `in 2 weeks` keeps the `in`.
        ("review due in 2 weeks", "review", "due in 2 weeks"),
        // A tag in front is not a lead-in and stays where it was typed.
        ("#priya due friday ship it", "#priya ship it", "due friday"),
    ];
    for (input, title, frag) in cases {
        assert_eq!(
            split(input),
            (title.to_string(), frag.to_string()),
            "{input}"
        );
    }
}

#[test]
fn a_lead_in_word_away_from_the_date_is_ordinary_prose() {
    let cases: &[(&str, &str)] = &[
        ("turn on the lights tomorrow", "turn on the lights"),
        ("look at the roof friday", "look at the roof"),
        ("drop by the shop 5pm", "drop by the shop"),
    ];
    for (input, title) in cases {
        assert_eq!(split(input).0, *title, "{input}");
    }
}

#[test]
fn a_lead_in_word_with_no_date_behind_it_is_not_a_parse() {
    for input in ["the rent is due", "call me on", "meet by"] {
        assert!(find(input, now()).is_none(), "{input}");
    }
}
