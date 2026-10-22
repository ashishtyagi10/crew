use super::{duration, repeat, span};
use crate::inbox::ics::parse;
use crate::inbox::ics::tests::cal;
use chrono::{Duration, NaiveDate};

#[test]
fn a_duration_in_its_own_notation() {
    let m = Duration::minutes;
    assert_eq!(duration("PT45M"), Some(m(45)));
    assert_eq!(duration("PT1H30M"), Some(m(90)));
    assert_eq!(duration("P1DT2H"), Some(m(26 * 60)));
    assert_eq!(duration("P2W"), Some(Duration::weeks(2)));
    assert_eq!((duration("1H"), duration("P1X")), (None, None));
}

#[test]
fn a_repeat_in_words() {
    for (rule, said) in [
        ("FREQ=WEEKLY;BYDAY=TU", "every week on Tuesday"),
        (
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE",
            "every 2 weeks on Monday and Wednesday",
        ),
        (
            "FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20261218T000000Z",
            "every week on Monday, Wednesday and Friday, until Dec 18",
        ),
        ("FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR", "every weekday"),
        ("FREQ=DAILY;COUNT=5", "every day, 5 times"),
        ("FREQ=MONTHLY;BYMONTHDAY=1", "every month"),
        ("FREQ=YEARLY", "every year"),
        ("FREQ=HOURLY", "repeats"),
    ] {
        assert_eq!(repeat(rule), said, "{rule}");
    }
}

#[test]
fn across_midnight_both_days_are_named_and_ending_at_midnight_is_one_day() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
    let said = |ev: &str| span(&parse(&cal("", ev)).unwrap(), &chrono_tz::UTC, today).unwrap();
    let late = "DTSTART:20261106T220000\r\nDTEND:20261107T010000\r\n";
    assert_eq!(said(late), "Fri Nov 6, 22:00 \u{2013} Sat Nov 7, 01:00");
    let midnight = "DTSTART:20261106T230000\r\nDTEND:20261107T000000\r\n";
    assert_eq!(said(midnight), "Fri Nov 6 \u{b7} 23:00\u{2013}00:00");
}
