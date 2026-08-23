use super::*;

#[test]
fn parses_real_times_and_rejects_the_rest() {
    assert_eq!(parse_hhmm("07:00"), Some(420));
    assert_eq!(parse_hhmm("7:5"), Some(425));
    assert_eq!(parse_hhmm(" 23:59 "), Some(1439));
    assert_eq!(parse_hhmm("00:00"), Some(0));
    // Every rejection matters: each of these would otherwise land on some
    // arbitrary minute and quietly reshape the user's day.
    assert_eq!(parse_hhmm("24:00"), None);
    assert_eq!(parse_hhmm("07:60"), None);
    assert_eq!(parse_hhmm("7"), None);
    assert_eq!(parse_hhmm("noon"), None);
    assert_eq!(parse_hhmm(""), None);
    assert_eq!(parse_hhmm("-1:00"), None);
}

#[test]
fn the_default_window_is_day_at_noon_and_night_at_midnight() {
    let (f, t) = (DEFAULT_FROM, DEFAULT_TO);
    assert!(is_day(12 * 60, f, t), "noon must be day");
    assert!(is_day(7 * 60, f, t), "the window is inclusive at the start");
    assert!(is_day(18 * 60 + 59, f, t));
    assert!(!is_day(19 * 60, f, t), "the window is exclusive at the end");
    assert!(!is_day(0, f, t));
    assert!(!is_day(6 * 60 + 59, f, t));
    assert!(!is_day(23 * 60, f, t));
}

#[test]
fn a_window_that_wraps_past_midnight_spans_it() {
    // 20:00 → 06:00: daylight is the night shift's day.
    let (f, t) = (20 * 60, 6 * 60);
    assert!(is_day(22 * 60, f, t));
    assert!(is_day(0, f, t));
    assert!(is_day(5 * 60 + 59, f, t));
    assert!(!is_day(6 * 60, f, t));
    assert!(!is_day(12 * 60, f, t));
    assert!(!is_day(19 * 60 + 59, f, t));
}

#[test]
fn an_empty_window_is_never_day() {
    for m in [0u16, 420, 720, 1439] {
        assert!(!is_day(m, 600, 600), "minute {m} must not be day");
    }
}

#[test]
fn is_day_now_agrees_with_is_day_on_the_same_clock() {
    use chrono::Timelike;
    let t = chrono::Local::now();
    let now = (t.hour() * 60 + t.minute()) as u16;
    // An always-day and a never-day window pin the answer regardless of when
    // the suite runs, so this asserts the wall-clock read really feeds `is_day`
    // rather than that the machine happens to be in daylight.
    assert!(is_day_now(0, 1439) == is_day(now, 0, 1439));
    assert!(!is_day_now(600, 600));
    assert_eq!(
        is_day_now(DEFAULT_FROM, DEFAULT_TO),
        is_day(now, DEFAULT_FROM, DEFAULT_TO)
    );
}
