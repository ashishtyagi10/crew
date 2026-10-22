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
use crate::chatbody::{plain, CardCell, CardLine, Color};

/// The bar down the card's left edge.
const BAR: char = '\u{258e}';

/// The card's lines for `m`, with a row of air after; nothing when the mail
/// carries no invite crew can read.
pub(crate) fn lines(lay: &mut Lay, m: &Mail, k: &Key) {
    let Some(inv) = m.calendar.as_deref().and_then(super::parse) else {
        return;
    };
    let t = crew_theme::theme();
    let width = k.width.saturating_sub(3).max(8);
    let mut say = |text: &str, fg: Color, bold: bool| {
        for l in super::super::draw::wrap(text, width) {
            lay.lines.push(bar(words(&l, fg, bold), k));
        }
    };
    let title = match inv.summary.trim() {
        "" => "(no title)",
        s => s,
    };
    match inv.cancelled {
        true => say(
            &format!("cancelled \u{b7} {title}"),
            crew_theme::readable::danger(&t),
            true,
        ),
        false => say(title, k.ink, true),
    }
    let today = chrono::Local::now().date_naive();
    let mut when = super::icswhen::span(&inv, &chrono::Local, today).unwrap_or_default();
    if let Some(r) = &inv.rrule {
        when = format!("{when} \u{b7} {}", super::icswhen::repeat(r));
    }
    say(&when, k.ink, false);
    let place = inv.location.trim();
    if !place.is_empty() && inv.link.as_deref().is_none_or(|l| !place.contains(l)) {
        say(place, k.muted, false);
    }
    if let Some(who) = asked(&inv) {
        say(&who, k.muted, false);
    }
    if let Some(url) = &inv.link {
        lay.lines
            .push(bar(link(url, crew_theme::readable::link(&t)), k));
    }
    lay.lines.push(Vec::new());
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
