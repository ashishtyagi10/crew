//! The due date a thread states (`todoguess`): a calendar invite's start;
//! else a date after "by", "before", "until", "due", "deadline" or "no
//! later than" in the newest mail that has one, then in the subject, each
//! read as of the day its mail was sent. A day's or a week's end is read as
//! people mean it: "EOD" is that day at 5pm, "end of the week" that Friday at
//! 5pm, "next Friday" the Friday of the week after. A date that reads two
//! ways ("10/11") is not a date `/todo` reads, so it is left blank. A month
//! and day is the one nearest the mail's day, so a deadline already passed
//! when it was written stays passed.
use chrono::{Datelike, Duration, Local, NaiveDateTime};

use super::super::super::api::Mail;
use super::super::Guess;
use super::{due_text, sent, whose};
use crate::todopane::duedate;
#[path = "todoeod.rs"]
mod eod;
use eod::end_of;

const KEYS: [&str; 7] = [
    "no later than ",
    "deadline: ",
    "deadline ",
    "before ",
    "until ",
    "due ",
    "by ",
];
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
    // A key starts a word: "lobby at 3pm" holds no "by", "overdue" no "due".
    let starts_word = |i: usize| !lower[..i].ends_with(|c: char| c.is_alphanumeric());
    KEYS.iter()
        .flat_map(|k| lower.match_indices(k).map(move |(i, _)| (*k, i)))
        .filter(|&(_, i)| starts_word(i))
        .map(|(k, i)| (k, i + k.len()))
        .filter_map(|(k, i)| {
            // Sentence punctuation is not part of a date: "by Friday?".
            // "the" first, so "by the end of the day" keeps all its words.
            let after = lower[i..].trim_start();
            let after = after.strip_prefix("the ").unwrap_or(after);
            let words: Vec<&str> = after
                .split_whitespace()
                .take(5)
                .map(|w| w.trim_end_matches(['.', ',', ';', ':', '!', '?', ')']))
                .collect();
            let phrase = words.join(" ");
            // "deadline is Friday", "due date: Oct 30".
            let phrase = ["date ", "is "]
                .iter()
                .fold(phrase.as_str(), |p, w| p.strip_prefix(w).unwrap_or(p));
            if let Some((due, timed, said)) = end_of(phrase, at) {
                return Some((due, timed, format!("{k}{said}")));
            }
            // The date comes right after its key: "By the way, Friday works"
            // and "signed by Ann on Oct 15" set no deadline. Cut back a word
            // at a time, so "by Friday, Oct 23" still reads Friday; a weekday
            // may lead the date it names: "by Friday, October 30".
            let led = |h: &duedate::DueHit, s: &str| {
                let lead: String = s.chars().take(h.start).collect();
                let lead = lead.trim_end();
                let whole = duedate::find(lead, at)
                    .is_some_and(|w| w.start == 0 && w.end == lead.chars().count());
                h.start == 0 || whole
            };
            let cuts: Vec<usize> = (phrase.match_indices(' ').map(|(j, _)| j))
                .chain([phrase.len()])
                .collect();
            let h = (cuts.into_iter().rev()).find_map(|j| {
                let s = &phrase[..j];
                duedate::find(s, at).filter(|h| led(h, s))
            })?;
            let said: String = phrase.chars().skip(h.start).take(h.end - h.start).collect();
            // "was due by", "were due": an overdue notice.
            let before = lower[..i - k.len()].split_whitespace().rev().take(2);
            let was = before.into_iter().any(|w| w == "was" || w == "were");
            Some((
                nearest(h.due, at, &said, was),
                h.has_time,
                format!("{k}{said}"),
            ))
        })
        .min_by_key(|d| d.0)
}

/// A month and day with no year, as the one nearest `at`. `/todo` reads
/// "oct 15" as the next one, right for a todo typed today, wrong for a mail
/// of Oct 20 saying it "was due by Oct 15": that landed a year out. Only a
/// month and day can roll half a year ahead; "in 300 days" and a written
/// year ("2027-10-15") mean what they say. And only a deadline that reads as
/// passed: an overdue notice (`was` due), or one weeks back. "Applications
/// are due by May 15", in October, is next May's, not five months overdue.
fn nearest(due: NaiveDateTime, at: NaiveDateTime, said: &str, was: bool) -> NaiveDateTime {
    let written = said.starts_with("in ") || said.contains('-');
    let late = |back| was || at - back <= Duration::days(LATE_DAYS);
    match due.with_year(due.year() - 1) {
        Some(back) if !written && late(back) && at - back < due - at => back,
        _ => due,
    }
}

/// How far back a month and day may be read as passed with no "was" to say so.
const LATE_DAYS: i64 = 60;

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

/// `Fri Oct 24`, its time when one was said, and its year when it is not
/// this one: "Thu Oct 15" said nothing of which October.
fn day(due: NaiveDateTime, timed: bool) -> String {
    let year = due.year() != duedate::now_local().year();
    due.format(match (timed, year) {
        (true, false) => "%a %b %-d %H:%M",
        (false, false) => "%a %b %-d",
        (true, true) => "%a %b %-d %Y %H:%M",
        (false, true) => "%a %b %-d %Y",
    })
    .to_string()
}

#[cfg(test)]
#[path = "tododue_tests.rs"]
mod tests;
