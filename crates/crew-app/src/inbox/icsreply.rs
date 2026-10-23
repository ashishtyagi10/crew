//! Your answer to an invite, as a mail its organizer's calendar reads (iTIP
//! `REPLY` over iMIP, RFC 5546 and 6047): `METHOD:REPLY`, the event's UID
//! and SEQUENCE, its organizer, and you as the one attendee with your
//! answer, beside a line of words for anyone reading it by eye. Outlook,
//! Apple Calendar and Google Calendar all take one. It needs only the
//! permission the inbox already has, so it is how an invite is answered
//! whenever Google Calendar is not the way (`icscal`).
use base64::Engine;

use super::Invite;
use crate::inbox::mime::{self, Node};

/// What you said to an invite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Answer {
    Yes,
    No,
    Maybe,
}

impl Answer {
    /// The iCalendar `PARTSTAT`.
    pub(crate) fn partstat(self) -> &'static str {
        ["ACCEPTED", "DECLINED", "TENTATIVE"][self as usize]
    }

    /// Google Calendar's `responseStatus`.
    pub(crate) fn google(self) -> &'static str {
        ["accepted", "declined", "tentative"][self as usize]
    }

    /// `accepted`, as in "you accepted"; Outlook's subject word.
    pub(crate) fn said(self) -> &'static str {
        ["accepted", "declined", "said maybe to"][self as usize]
    }

    fn subject(self) -> &'static str {
        ["Accepted", "Declined", "Tentative"][self as usize]
    }
}

/// The address you were asked at: the attendee that is you (an alias of
/// your account counts only when it is the very address), else your own.
fn asked_as(inv: &Invite, me: &str) -> String {
    let mine = inv
        .attendees
        .iter()
        .find(|a| a.email.eq_ignore_ascii_case(me));
    mine.map_or_else(|| me.to_ascii_lowercase(), |a| a.email.clone())
}

/// The `REPLY` calendar, stamped `stamp` (`YYYYMMDDTHHMMSSZ`).
pub(crate) fn calendar(inv: &Invite, me: &str, a: Answer, stamp: &str) -> Option<String> {
    let org = inv.organizer.as_ref()?;
    let clean = |s: &str| s.replace(['\r', '\n'], " ");
    let text = |s: &str| {
        clean(s)
            .replace('\\', "\\\\")
            .replace(';', "\\;")
            .replace(',', "\\,")
    };
    let lines = [
        "BEGIN:VCALENDAR".to_string(),
        "PRODID:-//crew//inbox//EN".into(),
        "VERSION:2.0".into(),
        "METHOD:REPLY".into(),
        "BEGIN:VEVENT".into(),
        format!("UID:{}", clean(&inv.uid)),
        format!("SEQUENCE:{}", inv.sequence),
        format!("DTSTAMP:{stamp}"),
        format!("ORGANIZER:mailto:{}", clean(&org.email)),
        format!(
            "ATTENDEE;PARTSTAT={}:mailto:{}",
            a.partstat(),
            clean(&asked_as(inv, me))
        ),
        format!("SUMMARY:{}", text(&inv.summary)),
        "END:VEVENT".into(),
        "END:VCALENDAR".into(),
    ];
    // One occurrence's answer names it: without it the organizer reads the
    // REPLY as one for the whole series (RFC 5546 3.2.3), and declining a
    // moved meeting declined every one.
    let mut lines = Vec::from(lines);
    if let Some(at) = &inv.occurrence {
        lines.insert(6, format!("RECURRENCE-ID{}", clean(at)));
    }
    Some(lines.join("\r\n") + "\r\n")
}

/// The whole mail, base64url as Gmail's `messages.send` takes it: to the
/// organizer, `Accepted: Q4 planning`, the words, then the calendar.
pub(crate) fn mail(inv: &Invite, me: &str, a: Answer, stamp: &str) -> Option<(String, String)> {
    let org = inv.organizer.as_ref()?;
    let cal = calendar(inv, me, a, stamp)?;
    let to = crate::inbox::addr::header(&org.email);
    let subject = format!(
        "{}: {}",
        a.subject(),
        inv.summary.replace(['\r', '\n'], " ")
    );
    let head = vec![
        format!("To: {to}"),
        format!("Subject: {}", encoded(&subject)),
    ];
    let words = format!("{} {}: {}", asked_as(inv, me), a.said(), inv.summary);
    let tree = Node::multi(
        "alternative",
        vec![
            Node::text("plain", &words),
            Node::text("calendar; method=REPLY", &cal),
        ],
    );
    let raw = mime::message(&head, &tree);
    Some((
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw),
        org.email.clone(),
    ))
}

fn encoded(s: &str) -> String {
    match s.is_ascii() {
        true => s.to_string(),
        false => format!(
            "=?UTF-8?B?{}?=",
            base64::engine::general_purpose::STANDARD.encode(s)
        ),
    }
}

#[cfg(test)]
#[path = "icsreply_tests.rs"]
mod tests;
