//! What a row of the list can afford at its width, and what every row on
//! screen shares (goal 2026-10-20-inbox-list-look, items 7 and 11).
use super::wire::Thread;
use crate::chatwidth::str_w;

/// The unread dot's column: the gutter, left of the words.
pub(crate) const DOT: u16 = 1;
/// Where every row's words start, on both lines, and each day's name.
pub(crate) const TEXT: u16 = 3;
/// The widest the names may run, so the subject is never pushed off a row.
pub(crate) const WHO_CAP: usize = 24;

/// The width ladder (item 7): what a row keeps as its pane narrows. The
/// snippet goes first, then the count (and the paperclip with it), then all
/// but the newest sender, and last the time turns compact. Two lines carry
/// more, so they climb it later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Fit {
    pub snippet: bool,
    pub count: bool,
    pub all: bool,
    pub compact: bool,
}

impl Fit {
    pub(crate) fn at(cols: u16, two: bool) -> Self {
        let w = cols + if two { 16 } else { 0 };
        Self {
            snippet: w >= 64,
            count: w >= 52,
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

/// What follows the names, each after a space: Gmail's red `draft` when
/// the thread holds one, and the message count.
pub(crate) enum Tail {
    Draft,
    Count(String),
}

impl Tail {
    pub(crate) fn text(&self) -> &str {
        match self {
            Tail::Draft => "draft",
            Tail::Count(n) => n,
        }
    }
}

pub(crate) fn tail(th: &Thread, fit: Fit) -> Vec<Tail> {
    let mut out = Vec::new();
    if th.draft {
        out.push(Tail::Draft);
    }
    if fit.count && th.count > 1 {
        out.push(Tail::Count(th.count.to_string()));
    }
    out
}

/// The columns [`tail`] takes, its spaces included.
pub(crate) fn tail_w(tail: &[Tail]) -> u16 {
    tail.iter().map(|t| 1 + str_w(t.text()) as u16).sum()
}

/// The one-line names column for these rows: as wide as the widest names
/// and tail on screen, never a fixed gap, and at most a third of what the
/// names and the subject share. A third of the PANE let a quarter tile's
/// names take 12 columns and leave the subject 10 (`Photos fr…` beside
/// `Google Cloud`, 2026-11-05 shot): the subject is what tells threads
/// apart, and a long sender falls back to a first name before it is cut.
pub(crate) fn who_w(rows: &[Thread], cols: u16, me: &str) -> u16 {
    let fit = Fit::at(cols, false);
    // The widest names and tail: the names in `room`, or in `room` less
    // the tail (as `row` writes them in the column).
    let widest = |room: u16, less_tail: bool| {
        let w = |th: &Thread| {
            let tw = tail_w(&tail(th, fit));
            let room = usize::from(room.saturating_sub(if less_tail { tw } else { 0 }));
            str_w(&super::who::names(th, me, room, fit.all)) as u16 + tw
        };
        rows.iter().map(w).max().unwrap_or(0)
    };
    let full = widest(WHO_CAP as u16, false);
    let cap = full.min(shared(cols, fit) / 3).max(full.min(WHO_FLOOR));
    // Then as wide as the names really are in it: a sender that fell back
    // to its first name leaves the rest of the cap to the subject.
    widest(cap, true).min(cap)
}

/// The fewest columns the names give up to the subject: `Bo Park`,
/// `Cy Diaz` and `Stripe` stay whole on the narrowest pane.
const WHO_FLOOR: u16 = 8;

/// What the names and the subject share on a one-line row: the pane less
/// the gutter, the gap after the names, the air and the widest time
/// (`Oct 30`, or `5mo` compact) and the margin at the right.
fn shared(cols: u16, fit: Fit) -> u16 {
    let time = if fit.compact { 3 } else { 6 };
    cols.saturating_sub(TEXT + 3 + 2 + time + 2)
}

/// The attachment mark: a paperclip from the icon set when the font really
/// has one (`glyphs`), else a page.
pub(crate) fn clip() -> &'static str {
    match crate::glyphs::on() {
        true => "\u{f0c6}",
        false => "\u{2398}",
    }
}
