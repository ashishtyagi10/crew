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
/// and tail on screen, never a fixed gap, and at most a third of the pane.
pub(crate) fn who_w(rows: &[Thread], cols: u16, me: &str) -> u16 {
    let fit = Fit::at(cols, false);
    let widest = rows
        .iter()
        .map(|th| {
            let names = super::who::names(th, me, WHO_CAP, fit.all);
            str_w(&names) as u16 + tail_w(&tail(th, fit))
        })
        .max()
        .unwrap_or(0);
    widest.min(cols / 3)
}

/// The attachment mark: a paperclip from the icon set when the font really
/// has one (`glyphs`), else a page.
pub(crate) fn clip() -> &'static str {
    match crate::glyphs::on() {
        true => "\u{f0c6}",
        false => "\u{2398}",
    }
}
