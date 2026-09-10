//! A due date in another year says the year; `jan 5` alone read the same
//! for next month, next year and a year overdue.
use super::label_naive;
use chrono::NaiveDate;

fn at(y: i32, m: u32, d: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap()
}

#[test]
fn another_year_is_named_and_this_year_is_not() {
    let now = at(2026, 9, 10);
    assert_eq!(label_naive(at(2026, 11, 5), false, now), "nov 5");
    assert_eq!(label_naive(at(2027, 1, 5), false, now), "jan 5 2027");
    assert_eq!(label_naive(at(2025, 1, 5), false, now), "jan 5 2025");
    assert_eq!(label_naive(at(2027, 1, 5), true, now), "jan 5 2027 09:00");
}
