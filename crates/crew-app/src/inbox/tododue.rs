//! The due date a thread states (`todoguess`): a calendar invite's start;
//! else a date after "by", "before", "until", "due", "deadline" or "no
//! later than" in the newest mail that has one, then in the subject, each
//! read as of the day its mail was sent. A day's or a week's end is read as
//! people mean it: "EOD" is that day at 5pm, "end of the week" that Friday at
//! 5pm, "next Friday" the Friday of the week after. A date that reads two
//! ways ("10/11") is not a date `/todo` reads, so it is left blank.
use chrono::{Datelike, Duration, Local, NaiveDateTime};

use super::super::super::api::Mail;
use super::super::Guess;
use super::{due_text, sent, whose};
use crate::todopane::duedate;

const KEYS: [&str; 7] = [
    "no later than ",
    "deadline: ",
    "deadline ",
    "before ",
    "until ",
    "due ",
    "by ",
];
const EOD: [&str; 5] = [
    "end of the day",
    "end of day",
    "close of business",
    "eod",
    "cob",
];
const EOW: [&str; 3] = ["end of the week", "end of week", "eow"];

pub(super) fn guess(subject: &str, mails: &[Mail]) -> Guess {
    if let Some(g) = mails.iter().rev().find_map(invite) {
        return g;
    }
    let said = |(due, timed, words): (NaiveDateTime, bool, String), from: String| Guess {
        text: due_text(due, timed),
        why: format!(
            "{} \u{b7} \u{201c}{words}\u{201d} in {from}",
            day(due, timed)
        ),
    };
    for m in mails.iter().rev() {
        if let Some(d) = deadline(super::words(m), sent(m)) {
            return said(d, whose(m));
        }
    }
    let at = mails.last().map_or_else(duedate::now_local, sent);
    deadline(subject, at).map_or_else(Guess::default, |d| said(d, "the subject".into()))
}

/// The earliest deadline `text` states, as of `at`: when, whether a time was
/// said, and the words it was read from ("by friday").
pub(crate) fn deadline(text: &str, at: NaiveDateTime) -> Option<(NaiveDateTime, bool, String)> {
    let lower = text.to_lowercase();
    KEYS.iter()
        .flat_map(|k| lower.match_indices(k).map(move |(i, _)| (*k, i + k.len())))
        .filter_map(|(k, i)| {
            // Sentence punctuation is not part of a date: "by Friday?".
            let words: Vec<&str> = lower[i..]
                .split_whitespace()
                .take(4)
                .map(|w| w.trim_end_matches(['.', ',', ';', ':', '!', '?', ')']))
                .collect();
            let phrase = words.join(" ");
            let phrase = phrase.strip_prefix("the ").unwrap_or(&phrase);
            if let Some((due, timed, said)) = end_of(phrase, at) {
                return Some((due, timed, format!("{k}{said}")));
            }
            let h = duedate::find(phrase, at)?;
            let said: String = phrase.chars().skip(h.start).take(h.end - h.start).collect();
            Some((h.due, h.has_time, format!("{k}{said}")))
        })
        .min_by_key(|d| d.0)
}

/// A day's or week's end, or "next {weekday}", at the start of `phrase`.
fn end_of(phrase: &str, at: NaiveDateTime) -> Option<(NaiveDateTime, bool, String)> {
    let starts = |w: &str| {
        phrase.starts_with(w)
            && phrase[w.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric())
    };
    let five = |d: chrono::NaiveDate| d.and_hms_opt(17, 0, 0);
    if let Some(w) = EOD.iter().find(|w| starts(w)) {
        return Some((five(at.date())?, true, w.to_string()));
    }
    if let Some(w) = EOW.iter().find(|w| starts(w)) {
        let ahead = (4 + 7 - at.weekday().num_days_from_monday()) % 7;
        return Some((
            five(at.date() + Duration::days(ahead.into()))?,
            true,
            w.to_string(),
        ));
    }
    let wd = phrase.strip_prefix("next ")?.split_whitespace().next()?;
    let this = duedate::find(wd, at)
        .filter(|h| h.end == wd.chars().count())?
        .due;
    let days = (this.date() - at.date()).num_days();
    // "next Friday" said on a Tuesday is not this week's Friday.
    let same_week = days < 7 - i64::from(at.weekday().num_days_from_monday());
    let due = this + Duration::days(if days == 0 || same_week { 7 } else { 0 });
    Some((due, false, format!("next {wd}")))
}

/// An invite's start as the due date, in your time, unless it was
/// cancelled: read as the invite card reads it (`ics`), a named zone's time
/// in that zone.
fn invite(m: &Mail) -> Option<Guess> {
    use crate::inbox::ics::{self, icstz::Placed};
    let inv = ics::parse(m.calendar.as_deref()?).filter(|i| !i.cancelled)?;
    let (due, timed) = match ics::icstz::place(inv.start.as_ref()?, &inv.zones) {
        Placed::Day(d) => (d.and_hms_opt(9, 0, 0)?, false),
        Placed::At(t, _) => (t.with_timezone(&Local).naive_local(), true),
        Placed::Floating(t) | Placed::Unplaced(t, _) => (t, true),
    };
    Some(Guess {
        text: due_text(due, timed),
        why: format!("{} \u{b7} the invite\u{2019}s start", day(due, timed)),
    })
}

/// `Fri Oct 24`, and its time when one was said.
fn day(due: NaiveDateTime, timed: bool) -> String {
    due.format(if timed {
        "%a %b %-d %H:%M"
    } else {
        "%a %b %-d"
    })
    .to_string()
}
