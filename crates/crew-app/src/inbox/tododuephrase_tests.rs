//! The phrasings a deadline is said in, read as of the day the mail was
//! sent: a weekday before its date, "is" after the key, a day's end, a day
//! before the key.
use super::deadline;
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

/// "the end of the day" is read whole, and a day's end takes the date after
/// it however many words it is: "by the end of the day" set no due, and
/// "EOD Oct 30" was the mail's own day.
#[test]
fn the_end_of_the_day_is_read() {
    let at = |s| deadline(s, tue()).map(|d| d.0.format("%Y-%m-%d %H:%M").to_string());
    for (mail, due) in [
        ("Please send it by the end of the day.", "2026-10-20 17:00"),
        ("Please send it by the end of the week.", "2026-10-23 17:00"),
        (
            "Please send it by end of the day tomorrow.",
            "2026-10-21 17:00",
        ),
        ("Please send it by the end of Friday.", "2026-10-23 17:00"),
        ("Please send it by EOD Oct 30.", "2026-10-30 17:00"),
        ("Please send it by EOD next Friday.", "2026-10-30 17:00"),
        ("Need the slides by EOD please.", "2026-10-20 17:00"),
    ] {
        assert_eq!(at(mail).as_deref(), Some(due), "{mail}");
    }
    assert_eq!(at("Please send it by the end of the month."), None);
}

/// A day said just before the key is the day of the key's time: "due Friday
/// by 5pm" and "tomorrow by 10am" were five or ten on the day of the mail.
#[test]
fn a_day_before_the_key_is_the_times_day() {
    let read = |s| {
        let d = deadline(s, tue()).unwrap();
        (d.0.format("%Y-%m-%d %H:%M").to_string(), d.2)
    };
    assert_eq!(
        read("The report is due Friday by 5pm."),
        ("2026-10-23 17:00".into(), "due friday by 5pm".into())
    );
    assert_eq!(
        read("Can you send it tomorrow by 10am?"),
        ("2026-10-21 10:00".into(), "tomorrow by 10am".into())
    );
    assert_eq!(read("Please send it by 5pm.").0, "2026-10-20 17:00");
    assert_eq!(
        read("See you Friday. By 5pm, please.").0,
        "2026-10-20 17:00"
    );
}
