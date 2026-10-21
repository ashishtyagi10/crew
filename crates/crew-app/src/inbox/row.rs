//! One thread in the list (goal 2026-10-20-inbox-list-look, phase 1).
//!
//! Two lines when the pane has room: who wrote, the message count as a quiet
//! chip, a star and the time at the right; then the subject and, in muted
//! ink, the snippet. One line when it has not: the same things across. An
//! unread thread is bright ink and bold, a read one recedes to muted ink, and
//! the dot that says unread has the gutter to itself, so no two markers ever
//! share a cell. The selected row wears the wash from edge to edge, and every
//! colour on it is lifted to read against it.
use crew_render::{CellView, Paint};

use super::line::Line;
use super::wire::Thread;
use crate::chatwidth::str_w;

type Rgb = (u8, u8, u8);

/// The column the names (and a one-line row's whole text) start at.
pub(crate) const NAMES: u16 = 3;
/// The widest the names may run, so the subject is never pushed off a row.
const WHO_CAP: usize = 24;
/// On a two-line row the names have a line of their own.
const WHO_CAP_TWO: usize = 36;

/// The width ladder (item 7): what a row keeps as its pane narrows. The
/// snippet goes first, then the count chip, then all but the newest sender,
/// and last the time turns compact. Two lines carry more, so they climb it
/// later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Fit {
    pub snippet: bool,
    pub chip: bool,
    pub all: bool,
    pub compact: bool,
}

impl Fit {
    pub(crate) fn at(cols: u16, two: bool) -> Self {
        let w = cols + if two { 16 } else { 0 };
        Self {
            snippet: w >= 64,
            chip: w >= 52,
            all: w >= 44,
            compact: w < 36,
        }
    }
}

/// What every row on screen shares.
pub(crate) struct Ctx<'a> {
    pub cols: u16,
    pub two: bool,
    /// The account's address, for `me`.
    pub me: &'a str,
    pub now: i64,
    /// A one-line row's names column: the widest on screen ([`who_w`]).
    pub who_w: u16,
}

fn chip(th: &Thread, fit: Fit) -> Option<String> {
    (fit.chip && th.count > 1).then(|| th.count.to_string())
}

fn chip_w(th: &Thread, fit: Fit) -> u16 {
    chip(th, fit).map_or(0, |c| 1 + str_w(&c) as u16)
}

/// The one-line names column for these rows: as wide as the widest names
/// and chip on screen, never a fixed gap, and at most a third of the pane.
pub(crate) fn who_w(rows: &[Thread], cols: u16, me: &str) -> u16 {
    let fit = Fit::at(cols, false);
    let widest = rows
        .iter()
        .map(|th| str_w(&super::who::names(th, me, WHO_CAP, fit.all)) as u16 + chip_w(th, fit))
        .max()
        .unwrap_or(0);
    widest.min(cols / 3)
}

/// The thread at `row` (and `row + 1` on two lines).
pub(crate) fn cells(out: &mut Vec<CellView>, th: &Thread, row: u16, selected: bool, x: &Ctx) {
    let t = crew_theme::theme();
    let wash = selected.then(|| crew_theme::readable::selection_bg(&t));
    let ink = |c: Rgb| match wash {
        Some(w) => crew_theme::readable::against(c, w, crew_theme::contrast::text_floor()),
        None => c,
    };
    let fit = Fit::at(x.cols, x.two);
    let strong = ink(if th.unread { t.ink } else { t.text_muted });
    let (muted, accent) = (ink(t.text_muted), ink(crate::palette::accent()));
    let end = x.cols.saturating_sub(1);
    let mut top = Line::new(end);
    if th.unread {
        top.put(1, 2, "\u{25cf}", accent, false);
    }
    let age = match fit.compact {
        true => super::age::short(th.at_ms, x.now),
        false => super::age::said(th.at_ms, x.now),
    };
    let mut right = top.right(NAMES, end, &age, strong, th.unread);
    if th.starred {
        right = top.right(NAMES, right.saturating_sub(1), "\u{2605}", accent, false);
    }
    let stop = right.saturating_sub(1);
    let cw = chip_w(th, fit);
    let room = match x.two {
        true => usize::from(stop.saturating_sub(NAMES + cw)).min(WHO_CAP_TWO),
        false => usize::from(x.who_w.saturating_sub(cw)),
    };
    let who = super::who::names(th, x.me, room, fit.all);
    let after = top.put(NAMES, stop, &who, strong, th.unread);
    if let Some(n) = chip(th, fit) {
        top.chip(after + 1, stop, &n, muted);
    }
    let subject = match th.subject.trim() {
        "" => "(no subject)",
        s => s,
    };
    let chip_bg = crate::chatink::code_bg();
    let mut second = x.two.then(|| Line::new(end));
    let (at, stop) = match x.two {
        true => (NAMES, end),
        false => (NAMES + x.who_w + 2, stop),
    };
    let body = second.as_mut().unwrap_or(&mut top);
    let after = body.put(at, stop, subject, strong, th.unread);
    if fit.snippet && !th.snippet.is_empty() && after + 4 < stop {
        let snippet = format!(" \u{2014} {}", th.snippet);
        body.put(after, stop, &snippet, muted, false);
    }
    top.emit(out, row, wash, chip_bg);
    if let Some(body) = second {
        body.emit(out, row + 1, wash, chip_bg);
    }
}

/// The selected row's accent bar at its left edge, over the wash, on every
/// line the row has.
pub(crate) fn bar(row: u16, lines: u16) -> Paint {
    let accent = crate::palette::accent();
    Paint::solid(
        0.15,
        f32::from(row) + 0.12,
        0.3,
        f32::from(lines) - 0.24,
        accent,
    )
}

#[cfg(test)]
#[path = "row_tests.rs"]
mod tests;
