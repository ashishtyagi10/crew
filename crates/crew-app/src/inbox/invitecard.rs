//! The invite card at the top of a mail that carries a meeting invite (goal
//! 2026-10-22-inbox-calendar, phase 1): what it is, when in your time (and in
//! the organizer's, when their clocks differ), how often, where or how to
//! join, and who asked. A cancelled one says so first. Drawn as the mail's
//! own lines behind an accent bar, so it scrolls, wraps and takes a Cmd+click
//! exactly as the words under it do.
use std::sync::Arc;

use super::super::api::Mail;
use super::super::reader::Key;
use super::super::readmail::Lay;
use super::super::Reading;
use crate::chatbody::{plain, CardCell, CardLine, Color};

/// The bar down the card's left edge.
const BAR: char = '\u{258e}';

/// The card's lines for `m`, with a row of air after; nothing when the mail
/// carries no invite crew can read.
pub(crate) fn lines(lay: &mut Lay, m: &Mail, r: &Reading, k: &Key) {
    let Some(inv) = m.calendar.as_deref().and_then(super::parse) else {
        return;
    };
    // `c` and `y` act on the thread's newest invite: its card says so.
    let mails = r.mails.as_deref().unwrap_or_default();
    let newest = mails.iter().rev().find_map(|m| m.calendar.as_deref()) == m.calendar.as_deref();
    let t = crew_theme::theme();
    let title = match inv.summary.trim() {
        "" => "(no title)",
        s => s,
    };
    match inv.cancelled {
        true => say(
            lay,
            k,
            &format!("cancelled \u{b7} {title}"),
            crew_theme::readable::danger(&t),
            true,
        ),
        false => say(lay, k, title, k.ink, true),
    }
    let today = chrono::Local::now().date_naive();
    let mut when = super::icswhen::span(&inv, &chrono::Local, today).unwrap_or_default();
    if let Some(r) = &inv.rrule {
        when = format!("{when} \u{b7} {}", super::icswhen::repeat(r));
    }
    say(lay, k, &when, k.ink, false);
    let place = inv.location.trim();
    if !place.is_empty() && inv.link.as_deref().is_none_or(|l| !place.contains(l)) {
        say(lay, k, place, k.muted, false);
    }
    if let Some(who) = asked(&inv) {
        say(lay, k, &who, k.muted, false);
    }
    if let Some(url) = &inv.link {
        lay.lines
            .push(bar(link(url, crew_theme::readable::link(&t)), k));
    }
    match inv.reply_line() {
        Some(said) => say(lay, k, &said, k.accent, false),
        None if newest => foot(lay, &inv, r, k),
        None => {}
    }
    lay.lines.push(Vec::new());
}

/// The newest card's last lines: where it stands, then its keys, or the
/// answer being given.
fn foot(lay: &mut Lay, inv: &super::Invite, r: &Reading, k: &Key) {
    let google = r.on_cal.as_ref().filter(|(u, _)| *u == inv.uid);
    let stands = match (google, &r.answered) {
        // Added after an answer by mail: Google's copy has none of yours.
        (Some((_, Some(s))), a) => Some(format!(
            "\u{2713} on your Google Calendar \u{b7} {}",
            said(a.as_ref().filter(|_| s == "needsAction").unwrap_or(s))
        )),
        (_, Some(s)) => Some(said(s).to_string()),
        _ => None,
    };
    if let Some(line) = stands {
        say(lay, k, &line, k.accent, false);
    }
    let who = inv.organizer.as_ref().map(|o| match o.name.trim() {
        "" => o.email.clone(),
        n => n.to_string(),
    });
    let placed = inv
        .start
        .as_ref()
        .map(|s| super::icstz::place(s, &inv.zones));
    let unplaced = matches!(placed, Some(super::icstz::Placed::Unplaced(..)));
    let keys = match (r.answering, &who) {
        (true, Some(w)) => {
            format!("answer {w}: y yes \u{b7} n no \u{b7} m maybe \u{b7} esc not now")
        }
        _ if inv.cancelled || unplaced => return,
        (_, Some(_)) => "c adds it to your calendar \u{b7} y answers it".into(),
        (_, None) => "c adds it to your calendar".into(),
    };
    let bold = r.answering;
    say(lay, k, &keys, if bold { k.ink } else { k.muted }, bold);
}

/// Google's word for an answer, as the card says it.
fn said(s: &str) -> &'static str {
    match s {
        "accepted" => "you accepted",
        "declined" => "you declined",
        "tentative" => "you said maybe",
        _ => "you have not answered",
    }
}

/// What the card shows besides the invite, for the layout's key: the
/// answer being given, and what Google Calendar and you said.
pub(crate) fn key(r: &Reading) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (r.answering, &r.on_cal, &r.answered).hash(&mut h);
    h.finish()
}

/// `Ann Lee · 6 invited`: the organizer, and how many others were asked.
fn asked(inv: &super::Invite) -> Option<String> {
    let org = inv.organizer.as_ref();
    let others = inv
        .attendees
        .iter()
        .filter(|a| org.is_none_or(|o| o.email != a.email))
        .count();
    let name = org.map(|o| match o.name.trim() {
        "" => o.email.clone(),
        n => n.to_string(),
    });
    match (name, others) {
        (Some(n), 0) => Some(n),
        (Some(n), k) => Some(format!("{n} \u{b7} {k} invited")),
        (None, 0) => None,
        (None, k) => Some(format!("{k} invited")),
    }
}

/// The join link, shown without its scheme or its query (a meeting's
/// password rides there), and clickable to the whole of it.
fn link(url: &str, fg: Color) -> CardLine {
    let target: Arc<str> = url.into();
    let shown = url.trim_start_matches("https://");
    let shown = shown.split('?').next().unwrap_or(shown);
    shown
        .chars()
        .map(|c| CardCell {
            link: Some(Arc::clone(&target)),
            ..plain(c, fg, false)
        })
        .collect()
}

/// `text` in the card, wrapped to its width behind the bar.
fn say(lay: &mut Lay, k: &Key, text: &str, fg: Color, bold: bool) {
    for l in super::super::draw::wrap(text, k.width.saturating_sub(3).max(8)) {
        lay.lines.push(bar(words(&l, fg, bold), k));
    }
}

fn bar(mut text: CardLine, k: &Key) -> CardLine {
    let mut line = vec![
        plain(' ', k.ink, false),
        plain(BAR, k.accent, false),
        plain(' ', k.ink, false),
    ];
    line.append(&mut text);
    line
}

fn words(s: &str, fg: Color, bold: bool) -> CardLine {
    s.chars().map(|c| plain(c, fg, bold)).collect()
}

#[cfg(test)]
#[path = "invitecard_tests.rs"]
mod tests;
