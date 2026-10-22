use chrono::{NaiveDate, NaiveTime};

use super::{parts, said, Fields};
use crate::todopane::{duedate, item::TodoItem};

fn on(h: u32, m: u32) -> u64 {
    let day = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
    duedate::to_epoch_ms(day.and_hms_opt(h, m, 0).unwrap()).unwrap()
}

#[test]
fn a_due_reads_as_a_day_or_a_day_and_a_minute_and_comes_back_the_same() {
    let mut it = TodoItem {
        title: "ship it".into(),
        due_ms: Some(on(17, 30)),
        due_has_time: true,
        ..TodoItem::default()
    };
    let f = Fields::of(&it);
    assert_eq!(f.due.as_deref(), Some("2026-10-23T17:30"));
    assert_eq!(f.due_ms(), Some((on(17, 30), true)));
    it.due_ms = Some(on(9, 0));
    it.due_has_time = false;
    let f = Fields::of(&it);
    assert_eq!(f.due.as_deref(), Some("2026-10-23"));
    assert_eq!(f.due_ms(), Some((on(9, 0), false)), "a day sits at 9:00");
    it.due_ms = Some(on(6, 15)); // an old item: a day, at another hour
    assert_eq!(Fields::of(&it).due.as_deref(), Some("2026-10-23"));
}

#[test]
fn said_and_parts_are_one_another_backwards() {
    let day = NaiveDate::from_ymd_opt(2027, 1, 2).unwrap();
    let t = NaiveTime::from_hms_opt(8, 5, 0);
    assert_eq!(parts(&said(day, t)), Some((day, t)));
    assert_eq!(parts(&said(day, None)), Some((day, None)));
    assert_eq!(parts("soon"), None);
}

#[test]
fn titles_pair_regardless_of_case_and_edges_but_never_when_empty() {
    let f = |t: &str| Fields {
        title: t.into(),
        ..Fields::default()
    };
    assert!(f("Buy milk").same_title(&f("  buy MILK ")));
    assert!(!f("Buy milk").same_title(&f("Buy oat milk")));
    assert!(!f(" ").same_title(&f("")));
}
