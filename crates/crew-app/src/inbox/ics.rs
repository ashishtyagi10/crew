//! A meeting invite's iCalendar text (RFC 5545), read: the event a mail's
//! `text/calendar` part carries (`api.rs`, `Mail::calendar`), and the zones
//! it names (`icstz`). Only what the invite card and the todo card use is
//! kept; everything else in the file is passed over.
use chrono::{Duration, NaiveDate, NaiveDateTime};

use icsline::{join, person, when};
pub(crate) use icsline::{props, text, Prop};

#[path = "invitecard.rs"]
pub(crate) mod card;
#[path = "icsline.rs"]
mod icsline;
#[path = "icstz.rs"]
pub(crate) mod icstz;
#[path = "icswhen.rs"]
pub(crate) mod icswhen;

/// When an event starts or ends, as the file wrote it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum When {
    /// `VALUE=DATE`: a whole day.
    Day(NaiveDate),
    /// A time in UTC (`…Z`).
    Utc(NaiveDateTime),
    /// A time in a named zone (`TZID=`), or, with none, "wherever you are".
    Local(NaiveDateTime, Option<String>),
}

impl When {
    fn plus(&self, d: Duration) -> When {
        match self {
            When::Day(day) => When::Day(*day + d),
            When::Utc(t) => When::Utc(*t + d),
            When::Local(t, z) => When::Local(*t + d, z.clone()),
        }
    }
}

/// Someone the invite names: its organizer, or one of those asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Person {
    pub name: String,
    pub email: String,
    /// `PARTSTAT`: `ACCEPTED`, `DECLINED`, `TENTATIVE` or `NEEDS-ACTION`.
    pub answer: String,
}

/// The event an invite is about.
#[derive(Debug, Default)]
pub(crate) struct Invite {
    /// `REQUEST`, `CANCEL`, `PUBLISH`, `REPLY`; empty when the file says none.
    pub method: String,
    pub uid: String,
    pub sequence: u32,
    pub summary: String,
    pub start: Option<When>,
    /// `DTEND`, or the start plus `DURATION`.
    pub end: Option<When>,
    pub location: String,
    pub description: String,
    /// Where to join: Google's own `X-GOOGLE-CONFERENCE`, else the first
    /// meeting link in the place or the words.
    pub link: Option<String>,
    pub organizer: Option<Person>,
    pub attendees: Vec<Person>,
    pub rrule: Option<String>,
    pub cancelled: bool,
    /// Every `VTIMEZONE` block's lines, for a zone no database knows.
    pub zones: Vec<Vec<Prop>>,
}

/// The invite in `ics`: its first event that is not one changed occurrence
/// of a repeating one (`RECURRENCE-ID`), else its first event.
pub(crate) fn parse(ics: &str) -> Option<Invite> {
    let mut inv = Invite::default();
    let (mut events, mut stack, mut duration) = (Vec::new(), Vec::<String>::new(), None);
    for p in props(ics) {
        let in_zone = stack.iter().any(|s| s == "VTIMEZONE");
        match p.name.as_str() {
            "BEGIN" | "END" => {
                let (begin, what) = (p.name == "BEGIN", p.value.trim().to_ascii_uppercase());
                match (begin, what.as_str()) {
                    (true, "VTIMEZONE") => inv.zones.push(Vec::new()),
                    (true, "VEVENT") => events.push(Vec::new()),
                    _ => {}
                }
                // A zone keeps the edges of its STANDARD and DAYLIGHT blocks.
                if let Some(z) = inv
                    .zones
                    .last_mut()
                    .filter(|_| in_zone && what != "VTIMEZONE")
                {
                    z.push(p);
                }
                match begin {
                    true => stack.push(what),
                    false => drop(stack.pop()),
                }
            }
            "METHOD" if stack.len() == 1 => inv.method = p.value.trim().to_ascii_uppercase(),
            _ if in_zone => {
                if let Some(z) = inv.zones.last_mut() {
                    z.push(p);
                }
            }
            // An alarm's own lines (its DESCRIPTION, its TRIGGER) are not the event's.
            _ if stack.last().is_some_and(|s| s == "VEVENT") => {
                if let Some(e) = events.last_mut() {
                    e.push(p);
                }
            }
            _ => {}
        }
    }
    let pick = events
        .iter()
        .position(|e| !e.iter().any(|p| p.name == "RECURRENCE-ID"));
    for p in events.into_iter().nth(pick.unwrap_or(0))? {
        let v = p.value.trim();
        match p.name.as_str() {
            "UID" => inv.uid = v.to_string(),
            "SEQUENCE" => inv.sequence = v.parse().unwrap_or(0),
            "SUMMARY" => inv.summary = text(v),
            "DTSTART" => inv.start = when(&p),
            "DTEND" => inv.end = when(&p),
            "DURATION" => duration = icswhen::duration(v),
            "LOCATION" => inv.location = text(v),
            "DESCRIPTION" => inv.description = text(v),
            "ORGANIZER" => inv.organizer = Some(person(&p)),
            "ATTENDEE" => inv.attendees.push(person(&p)),
            "RRULE" => inv.rrule = Some(v.to_ascii_uppercase()),
            "STATUS" => inv.cancelled |= v.eq_ignore_ascii_case("CANCELLED"),
            "X-GOOGLE-CONFERENCE" => inv.link = Some(v.to_string()),
            _ => {}
        }
    }
    inv.cancelled |= inv.method == "CANCEL";
    inv.link = inv
        .link
        .take()
        .or_else(|| join(&inv.location).or_else(|| join(&inv.description)));
    if inv.end.is_none() {
        inv.end = inv.start.as_ref().zip(duration).map(|(s, d)| s.plus(d));
    }
    inv.start.is_some().then_some(inv)
}

#[cfg(test)]
#[path = "ics_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "icscorpus_tests.rs"]
mod corpus;
