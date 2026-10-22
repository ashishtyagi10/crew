//! A reminder read as [`Fields`], and a due written back as Reminders
//! keeps one: date components, and an alarm when it has a time.
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use objc2::rc::Retained;
use objc2_event_kit::{EKAlarm, EKReminder};
use objc2_foundation::{NSArray, NSDate, NSDateComponentUndefined, NSDateComponents};

use super::fields::{parts, said, Fields};

pub(super) fn read(r: &EKReminder) -> (String, Fields) {
    unsafe {
        let f = Fields {
            title: r.title().to_string(),
            done: r.isCompleted(),
            due: r.dueDateComponents().and_then(|c| due_of(&c)),
        };
        (r.calendarItemIdentifier().to_string(), f)
    }
}

/// A due's components as [`Fields::due`] says them.
fn due_of(c: &NSDateComponents) -> Option<String> {
    let n = |v: isize| (v != NSDateComponentUndefined).then_some(v);
    let u = |v: isize| u32::try_from(n(v)?).ok();
    let day = NaiveDate::from_ymd_opt(
        i32::try_from(n(c.year())?).ok()?,
        u(c.month())?,
        u(c.day())?,
    )?;
    let at = u(c.hour()).and_then(|h| NaiveTime::from_hms_opt(h, u(c.minute()).unwrap_or(0), 0));
    Some(said(day, at))
}

pub(super) fn components(due: &str) -> Option<Retained<NSDateComponents>> {
    let (day, at) = parts(due)?;
    let c = NSDateComponents::new();
    c.setYear(day.year() as isize);
    c.setMonth(day.month() as isize);
    c.setDay(day.day() as isize);
    if let Some(t) = at {
        c.setHour(t.hour() as isize);
        c.setMinute(t.minute() as isize);
    }
    Some(c)
}

/// A due with a time rings then, as one set in Reminders does; a day alone
/// rings never. Set only as the due changes, so an alarm added on the phone
/// stands until it does.
pub(super) fn ring(r: &EKReminder, f: &Fields) {
    let at = f.due_ms().filter(|d| d.1).map(|d| d.0 as f64 / 1000.0);
    let alarms = at.map(|s| {
        let alarm =
            unsafe { EKAlarm::alarmWithAbsoluteDate(&NSDate::dateWithTimeIntervalSince1970(s)) };
        NSArray::from_retained_slice(&[alarm])
    });
    unsafe { r.setAlarms(alarms.as_deref()) }
}
