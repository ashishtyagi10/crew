//! Table tests for the due-date grammar. `now` is fixed at Wednesday
//! 2026-08-12 12:00 so every expectation is a concrete date, not a formula.
use super::*;

fn now() -> NaiveDateTime {
    let d = NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();
    assert_eq!(d.weekday(), chrono::Weekday::Wed, "the fixture must hold");
    d.and_hms_opt(12, 0, 0).unwrap()
}

fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, mo, d)
        .unwrap()
        .and_hms_opt(h, mi, 0)
        .unwrap()
}

/// (input, stripped title, due, has_time) — the grammar's positive table.
#[test]
fn the_grammar_parses_and_strips_each_form() {
    let cases: &[(&str, &str, NaiveDateTime, bool)] = &[
        (
            "pay rent tomorrow",
            "pay rent",
            at(2026, 8, 13, 9, 0),
            false,
        ),
        ("pay rent today", "pay rent", at(2026, 8, 12, 9, 0), false),
        // Friday is two days out; the time rides the date.
        (
            "ship build fri 5pm",
            "ship build",
            at(2026, 8, 14, 17, 0),
            true,
        ),
        (
            "ship build friday",
            "ship build",
            at(2026, 8, 14, 9, 0),
            false,
        ),
        // Today IS Wednesday — `wed` means today, not next week.
        ("standup wed", "standup", at(2026, 8, 12, 9, 0), false),
        ("dentist aug 15", "dentist", at(2026, 8, 15, 9, 0), false),
        ("dentist 15 aug", "dentist", at(2026, 8, 15, 9, 0), false),
        ("dentist aug 15th", "dentist", at(2026, 8, 15, 9, 0), false),
        // A month-day already past this year rolls to next year.
        ("cake feb 1", "cake", at(2027, 2, 1, 9, 0), false),
        ("review in 2 weeks", "review", at(2026, 8, 26, 9, 0), false),
        ("review in 3 days", "review", at(2026, 8, 15, 9, 0), false),
        (
            "release 2026-12-01",
            "release",
            at(2026, 12, 1, 9, 0),
            false,
        ),
        (
            "release 2026-12-01 17:00",
            "release",
            at(2026, 12, 1, 17, 0),
            true,
        ),
        // A bare time still ahead today lands today…
        ("call mom 5pm", "call mom", at(2026, 8, 12, 17, 0), true),
        ("call mom 12:30", "call mom", at(2026, 8, 12, 12, 30), true),
        // …and one already passed rolls to tomorrow.
        ("standup 9:30", "standup", at(2026, 8, 13, 9, 30), true),
        ("standup 12am", "standup", at(2026, 8, 13, 0, 0), true),
        ("lunch 12pm", "lunch", at(2026, 8, 13, 12, 0), true),
        (
            "time tomorrow 5:30pm",
            "time",
            at(2026, 8, 13, 17, 30),
            true,
        ),
        // Time-first also combines.
        ("5pm fri drinks", "drinks", at(2026, 8, 14, 17, 0), true),
    ];
    for (input, title, due, has_time) in cases {
        let hit = find(input, now()).unwrap_or_else(|| panic!("no parse: {input}"));
        assert_eq!(hit.due, *due, "{input}");
        assert_eq!(hit.has_time, *has_time, "{input}");
        assert_eq!(strip(input, hit.start, hit.end), *title, "{input}");
    }
}

#[test]
fn words_that_look_datelike_do_not_parse() {
    for input in [
        "may I help you",  // bare month name needs a day number
        "pay 5 bills",     // bare digits are not a time
        "meet at 25:00",   // invalid hour
        "meet at 13pm",    // invalid 12h hour
        "jan",             // month alone
        "in days",         // no count
        "in 0 days",       // zero count
        "aug 32",          // invalid day
        "totally fine",    // nothing at all
        "@friday standup", // an @token never date-parses
    ] {
        assert_eq!(find(input, now()), None, "{input:?} must not parse");
    }
}

#[test]
fn rightmost_then_longest_window_wins() {
    // Both `fri` and `tomorrow 5pm` parse; the rightmost fragment is taken
    // and ONLY it is stripped — `fri` stays title text.
    let input = "fri standup tomorrow 5pm";
    let hit = find(input, now()).unwrap();
    assert_eq!(hit.due, at(2026, 8, 13, 17, 0));
    assert!(hit.has_time);
    assert_eq!(strip(input, hit.start, hit.end), "fri standup");
}

#[test]
fn rightmost_beats_a_longer_window_further_left() {
    // `[tomorrow 5pm]` is the longer parse, but `fri` sits further right —
    // rightmost wins, and only IT is stripped.
    let input = "tomorrow 5pm standup fri";
    let hit = find(input, now()).unwrap();
    assert_eq!(hit.due, at(2026, 8, 14, 9, 0));
    assert!(!hit.has_time);
    assert_eq!(strip(input, hit.start, hit.end), "tomorrow 5pm standup");
}

#[test]
fn a_window_never_straddles_an_at_tag() {
    // `fri … 5pm` may not combine across `@home`; the rightmost single
    // token (`5pm`, still ahead today) wins instead.
    let input = "pack fri @home 5pm";
    let hit = find(input, now()).unwrap();
    assert_eq!(hit.due, at(2026, 8, 12, 17, 0));
    assert_eq!(strip(input, hit.start, hit.end), "pack fri @home");
}

#[test]
fn strip_normalises_the_leftover_whitespace() {
    let input = "water   plants   tomorrow  please";
    let hit = find(input, now()).unwrap();
    assert_eq!(strip(input, hit.start, hit.end), "water plants please");
}

#[test]
fn labels_read_humane() {
    let n = now();
    assert_eq!(label_naive(at(2026, 8, 12, 17, 0), true, n), "today 17:00");
    assert_eq!(label_naive(at(2026, 8, 12, 9, 0), false, n), "today");
    assert_eq!(label_naive(at(2026, 8, 13, 9, 0), false, n), "tomorrow");
    // Saturday is three days out → weekday shorthand.
    assert_eq!(label_naive(at(2026, 8, 15, 9, 0), false, n), "sat");
    assert_eq!(label_naive(at(2026, 8, 15, 8, 30), true, n), "sat 08:30");
    // A week+ out → month-day.
    assert_eq!(label_naive(at(2026, 9, 11, 9, 0), false, n), "sep 11");
    assert_eq!(label_naive(at(2026, 8, 11, 9, 0), false, n), "yesterday");
    assert_eq!(label_naive(at(2026, 8, 1, 9, 0), false, n), "aug 1");
}

#[test]
fn edit_text_round_trips_through_the_parser() {
    // What `e` puts back into the composer must re-parse to the same due.
    let due = at(2027, 3, 5, 9, 0);
    let ms = to_epoch_ms(due).unwrap();
    let txt = edit_text(ms, false).unwrap();
    assert_eq!(txt, "2027-03-05");
    let hit = find(&format!("title {txt}"), now()).unwrap();
    // Date-only re-parses onto the DEFAULT_HOUR it was stored at.
    assert_eq!(to_epoch_ms(hit.due), Some(ms));
    assert!(!hit.has_time);

    let due = at(2027, 3, 5, 17, 30);
    let ms = to_epoch_ms(due).unwrap();
    let txt = edit_text(ms, true).unwrap();
    assert_eq!(txt, "2027-03-05 17:30");
    let hit = find(&format!("title {txt}"), now()).unwrap();
    assert_eq!(to_epoch_ms(hit.due), Some(ms));
    assert!(hit.has_time);
}

#[test]
fn epoch_conversion_round_trips() {
    let d = at(2026, 8, 12, 15, 4);
    assert_eq!(from_epoch_ms(to_epoch_ms(d).unwrap()), Some(d));
}
