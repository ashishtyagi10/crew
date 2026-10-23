//! Your Google Calendar, for an invite (goal 2026-10-22-inbox-calendar,
//! phase 3), on the inbox's worker. With the `calendar.events` scope (asked
//! for by `/inbox calendar`): whether the invite is on your primary
//! calendar and what you answered there (`events.list` by its UID), adding
//! it (`events.import`, which keeps the UID, so an invite Gmail already
//! added is never added twice), and answering it there (`events.patch` with
//! `sendUpdates=all`, so Google tells the organizer). Without the scope, or
//! for an invite not on Google Calendar, an answer goes as a mail
//! (`icsreply`).
use serde_json::{json, Value};

pub(crate) use super::event::event;
use super::parse;
use super::reply::{self as icsreply, Answer};
use crate::inbox::api::Gmail;
use crate::inbox::query::enc;
use crate::inbox::worker::Got;

/// The permission `/inbox calendar` asks for beside Gmail's.
pub(crate) const SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
/// The console page that turns the Calendar API on in your own project.
pub(crate) const ENABLE: &str =
    "https://console.cloud.google.com/apis/enableflow;apiid=calendar-json.googleapis.com";
/// Your primary calendar's events.
pub(crate) const EVENTS: &str = "https://www.googleapis.com/calendar/v3/calendars/primary/events";

impl Gmail {
    /// A PATCH, with the one renewal a 401 earns (`api::send_all`'s rule).
    fn patch(&mut self, url: &str, body: Value) -> Result<Value, String> {
        use crate::inbox::fetch::{self, Fetch};
        for attempt in 0..2 {
            let token = self.token()?;
            let sent = fetch::patch(self.http.clone(), token, url.to_string(), body.clone());
            match self.rt.block_on(sent) {
                Err(Fetch::Expired) if attempt == 0 => self.access = None,
                Err(Fetch::Expired) => return Err("Google refused the sign-in (401)".into()),
                Err(Fetch::Failed(e)) => return Err(e),
                Ok(v) => return Ok(v),
            }
        }
        unreachable!("the second attempt always returns")
    }

    fn calendar_ok(&self) -> bool {
        self.grant.scope.split_whitespace().any(|s| s == SCOPE)
    }

    /// The event with this UID on your primary calendar: its id, and your
    /// `responseStatus` there.
    fn on_calendar(&mut self, uid: &str) -> Result<Option<(String, String)>, String> {
        let v = self.get(&format!("{}?iCalUID={}", self.cal, enc(uid)))?;
        let Some(e) = v["items"].as_array().and_then(|a| a.first()) else {
            return Ok(None);
        };
        let me = e["attendees"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|a| a["self"] == true);
        let said = me
            .and_then(|a| a["responseStatus"].as_str())
            .unwrap_or("needsAction");
        Ok(Some((
            e["id"].as_str().unwrap_or_default().to_string(),
            said.to_string(),
        )))
    }
}

/// `Ask::OnCalendar`: nothing back without the scope, or when the call
/// fails (the card simply says nothing of your calendar).
pub(crate) fn seen(g: &mut Gmail, ics: &str) -> Vec<Got> {
    let Some(inv) = parse(ics).filter(|i| !i.uid.is_empty() && g.calendar_ok()) else {
        return Vec::new();
    };
    match g.on_calendar(&inv.uid) {
        Ok(on) => vec![Got::OnCalendar(inv.uid, on.map(|(_, said)| said))],
        Err(_) => Vec::new(),
    }
}

/// `Ask::AddEvent`: the invite onto your primary calendar.
pub(crate) fn add(g: &mut Gmail, ics: &str) -> Vec<Got> {
    let Some(inv) = parse(ics) else {
        return vec![Got::Failed("crew cannot read this invite".into())];
    };
    let done = g.post(&format!("{}/import", g.cal), event(&inv));
    vec![match done {
        Ok(_) => Got::Calendar(format!("added to your Google Calendar: {}", inv.summary)),
        Err(e) => Got::Failed(format!("not added: {e}")),
    }]
}

/// `Ask::Answer`: on Google Calendar when the event is there and the scope
/// allows; else a `REPLY` mail to the organizer.
pub(crate) fn answer(g: &mut Gmail, ics: &str, a: Answer, me: &str) -> Vec<Got> {
    let Some(inv) = parse(ics) else {
        return vec![Got::Failed("crew cannot read this invite".into())];
    };
    let who = inv
        .organizer
        .as_ref()
        .map(|o| o.name.clone())
        .filter(|n| !n.is_empty());
    let who = who
        .or_else(|| inv.organizer.as_ref().map(|o| o.email.clone()))
        .unwrap_or_default();
    // An occurrence goes by mail: the event Google finds by its UID is the
    // whole series, and patching it answered every meeting in it.
    let on = match g.calendar_ok() && inv.occurrence.is_none() {
        true => g.on_calendar(&inv.uid).ok().flatten(),
        false => None,
    };
    let done = match on {
        Some((id, _)) => patch_answer(g, &id, me, a),
        None => {
            let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
            match icsreply::mail(&inv, me, a, &stamp) {
                Some((raw, _)) => g.post("/messages/send", json!({ "raw": raw })).map(|_| ()),
                None => Err("the invite names no organizer to answer".into()),
            }
        }
    };
    vec![match done {
        Ok(()) => Got::Answered(
            inv.uid,
            a.google().into(),
            format!("you {} \u{b7} {who} was told", a.said()),
        ),
        Err(e) => Got::Failed(format!("not answered: {e}")),
    }]
}

fn patch_answer(g: &mut Gmail, id: &str, me: &str, a: Answer) -> Result<(), String> {
    let url = format!("{}/{}", g.cal, enc(id));
    let ev = g.get(&url)?;
    let mut people = ev["attendees"].as_array().cloned().unwrap_or_default();
    if !mark_mine(&mut people, me, a) {
        return Err(format!(
            "you are not on its guest list as {me} \u{2014} it reached you another way (a \
             group, another address), so there was no answer to set"
        ));
    }
    g.patch(
        &format!("{url}?sendUpdates=all"),
        json!({ "attendees": people }),
    )
    .map(|_| ())
}

/// Set your answer on the guests who are you: Google's `self`, or your
/// address. Whether one was. With none, the patch changed nothing and the
/// foot still said the organizer "was told" (a group's invite, an alias).
fn mark_mine(people: &mut [serde_json::Value], me: &str, a: Answer) -> bool {
    let mut found = false;
    for p in people {
        let mine = p["self"] == true
            || p["email"]
                .as_str()
                .is_some_and(|e| e.eq_ignore_ascii_case(me));
        if mine {
            p["responseStatus"] = json!(a.google());
            found = true;
        }
    }
    found
}

#[cfg(test)]
#[path = "icscalmine_tests.rs"]
mod mine_tests;

#[cfg(test)]
#[path = "icscal_tests.rs"]
mod tests;
