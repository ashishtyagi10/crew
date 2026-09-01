//! Every failure here is a person's phrasing, so the tests are about what the parse REFUSES as
//! much as what it accepts: an alarm set for a time that has passed, or with a cadence crew
//! guessed at, is worse than one that was never set.
use super::*;

/// `duedate` resolves against the LOCAL clock, so a test that wants a future time has to ask for
/// one relative to today rather than hard-coding a date that will eventually be in the past.
fn now_ms() -> u64 {
    crate::chattime::unix_now_ms()
}

#[test]
fn the_time_comes_out_of_the_sentence_and_the_task_is_what_is_left() {
    let p = parse("tomorrow 9am brief me on the calendar", None, now_ms()).unwrap();
    assert_eq!(p.text, "brief me on the calendar");
    assert!(p.fire_ms > now_ms(), "and it is in the future");
    assert_eq!(p.repeat_secs, None, "no cadence asked for is a one-shot");
}

#[test]
fn a_cadence_becomes_seconds_on_the_wire() {
    let p = parse("tomorrow 7am the briefing", Some("daily"), now_ms()).unwrap();
    assert_eq!(p.repeat_secs, Some(86_400));
    let p = parse("tomorrow 7am the briefing", Some("every 30m"), now_ms()).unwrap();
    assert_eq!(p.repeat_secs, Some(1_800));
}

#[test]
fn a_cadence_crew_does_not_know_is_refused_by_name() {
    let e = parse("tomorrow 7am the briefing", Some("fortnightly"), now_ms()).unwrap_err();
    assert!(e.contains("fortnightly"), "{e}");
    assert!(e.contains("daily"), "and says what would work: {e}");
}

#[test]
fn a_sentence_with_no_time_in_it_says_what_a_time_looks_like() {
    let e = parse("brief me on the calendar", None, now_ms()).unwrap_err();
    assert!(e.contains("tomorrow 9am"), "{e}");
}

#[test]
fn a_time_with_nothing_to_do_at_it_is_not_an_intent() {
    let e = parse("tomorrow 9am", None, now_ms()).unwrap_err();
    assert!(e.contains("nothing to do"), "{e}");
}

#[test]
fn a_time_that_has_already_passed_is_refused_and_says_how_long_ago() {
    // The grammar resolves a bare time against today, so "9am" said at 10am is in the past.
    // Storing it would fire instantly, and a repeat would then fire on its cadence from a time
    // nobody chose.
    let future = parse("tomorrow 9am the forecast", None, now_ms()).unwrap();
    let e = parse("tomorrow 9am the forecast", None, future.fire_ms + 1).unwrap_err();
    assert!(e.contains("already passed"), "{e}");
    assert!(e.contains("ago"), "{e}");
}
