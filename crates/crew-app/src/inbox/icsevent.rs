//! An invite as Google Calendar's `events.import` takes it (`icscal`): its
//! UID, which keeps an invite Gmail already added from being added twice;
//! its times, a whole day as a date and an instant with the zone a series
//! repeats in; its place, words, organizer and people with their answers.
use serde_json::{json, Value};

use super::icstz::{place, Placed};
use super::{Invite, When};

/// The invite as `events.import` takes it.
pub(crate) fn event(inv: &Invite) -> Value {
    let start = inv
        .start
        .as_ref()
        .map(|w| time(w, inv))
        .unwrap_or(Value::Null);
    let end = inv
        .end
        .as_ref()
        .map_or_else(|| start.clone(), |w| time(w, inv));
    let person = |p: &super::Person| json!({ "email": p.email, "displayName": p.name, "responseStatus": status(&p.answer) });
    let mut v = json!({
        "iCalUID": inv.uid,
        "summary": inv.summary,
        "location": inv.location,
        "description": inv.description,
        "start": start,
        "end": end,
        "attendees": inv.attendees.iter().map(person).collect::<Vec<_>>(),
    });
    if let Some(o) = &inv.organizer {
        v["organizer"] = json!({ "email": o.email, "displayName": o.name });
    }
    if let Some(r) = &inv.rrule {
        v["recurrence"] = json!([format!("RRULE:{r}")]);
    }
    v
}

/// A start or end: a date, or an instant with the zone it repeats in.
fn time(w: &When, inv: &Invite) -> Value {
    match place(w, &inv.zones) {
        Placed::Day(d) => json!({ "date": d.format("%Y-%m-%d").to_string() }),
        Placed::At(t, z) => json!({
            "dateTime": t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "timeZone": z.map_or("UTC", |z| z.name()),
        }),
        Placed::Floating(t) | Placed::Unplaced(t, _) => json!({
            "dateTime": t.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "timeZone": iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".into()),
        }),
    }
}

/// An iCalendar `PARTSTAT` as Google Calendar says it.
fn status(partstat: &str) -> &'static str {
    match partstat {
        "ACCEPTED" => "accepted",
        "DECLINED" => "declined",
        "TENTATIVE" => "tentative",
        _ => "needsAction",
    }
}
