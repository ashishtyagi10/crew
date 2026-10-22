use super::{nth, place, tz, Placed};
use crate::inbox::ics::tests::{at, cal};
use crate::inbox::ics::{parse, When};
use chrono::{NaiveDate, TimeZone, Utc, Weekday};

#[test]
fn a_zone_is_found_by_its_name_its_tail_or_its_windows_name() {
    use chrono_tz::{America, Europe};
    assert_eq!(tz("America/New_York"), Some(America::New_York));
    assert_eq!(tz("\"Europe/Berlin\""), Some(Europe::Berlin));
    assert_eq!(
        tz("/mozilla.org/20050126_1/America/New_York"),
        Some(America::New_York)
    );
    assert_eq!(tz("Pacific Standard Time"), Some(America::Los_Angeles));
    assert_eq!(tz("W. Europe Standard Time"), Some(Europe::Berlin));
    assert_eq!(tz("Customized Time Zone"), None);
}

#[test]
fn the_nth_and_the_last_weekday_of_a_month() {
    let d = |m, d| NaiveDate::from_ymd_opt(2026, m, d);
    assert_eq!(nth(2026, 3, 2, Weekday::Sun), d(3, 8));
    assert_eq!(nth(2026, 3, -1, Weekday::Sun), d(3, 29));
    assert_eq!(nth(2026, 11, 1, Weekday::Sun), d(11, 1));
    assert_eq!(nth(2026, 10, -1, Weekday::Sun), d(10, 25));
    assert_eq!(nth(2026, 12, -1, Weekday::Thu), d(12, 31));
}

const ZONE: &str = "BEGIN:VTIMEZONE\r\nTZID:Customized Time Zone\r\nBEGIN:STANDARD\r\n\
DTSTART:16010101T020000\r\nTZOFFSETFROM:-0400\r\nTZOFFSETTO:-0500\r\n\
RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=1SU;BYMONTH=11\r\nEND:STANDARD\r\nBEGIN:DAYLIGHT\r\n\
DTSTART:16010101T020000\r\nTZOFFSETFROM:-0500\r\nTZOFFSETTO:-0400\r\n\
RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=2SU;BYMONTH=3\r\nEND:DAYLIGHT\r\nEND:VTIMEZONE\r\n";

/// Outlook names a zone of its own and sends its rules along.
#[test]
fn a_zone_no_database_knows_is_read_from_the_invite_s_own_rules() {
    for (start, utc) in [
        ("20261028T150000", "2026-10-28 19:00"),
        ("20261202T150000", "2026-12-02 20:00"),
        ("20260310T090000", "2026-03-10 13:00"),
        ("20260105T090000", "2026-01-05 14:00"),
    ] {
        let ev = format!("DTSTART;TZID=Customized Time Zone:{start}\r\n");
        let inv = parse(&cal(ZONE, &ev)).unwrap();
        let Placed::At(t, None) = place(inv.start.as_ref().unwrap(), &inv.zones) else {
            panic!("{start} was not placed");
        };
        assert_eq!(t.format("%Y-%m-%d %H:%M").to_string(), utc, "{start}");
    }
}

#[test]
fn a_zone_nothing_knows_stays_as_written_and_a_skipped_hour_reads_an_hour_on() {
    let mars = When::Local(at("20261109T100000"), Some("Mars/Olympus_Mons".into()));
    let unplaced = Placed::Unplaced(at("20261109T100000"), "Mars/Olympus_Mons".into());
    assert_eq!(place(&mars, &[]), unplaced);
    let gap = When::Local(at("20260308T023000"), Some("America/New_York".into()));
    let Placed::At(t, _) = place(&gap, &[]) else {
        panic!("New York is known");
    };
    assert_eq!(t, Utc.with_ymd_and_hms(2026, 3, 8, 7, 30, 0).unwrap());
}
