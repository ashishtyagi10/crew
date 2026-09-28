//! Where each part of a task row sits on a `cols`-wide block — number and
//! glyph, the words, the span bar, the spend count at the right end — and
//! what a narrow block gives up to keep the words: the bar first (below a
//! fixed width, `chatswarmspan::bar_cols`), then every row's count, then the
//! words' own sacrifices (`chatswarmrows::level`). A count is worth less
//! than any word on the row — the status line carries the run's total — so
//! it stays only while every row's words fit whole beside it, and it goes
//! from every row at once, like the specialist and the deps do.
use crate::chatswarm::SwarmStatus;

/// Left inset, under the status line's spinner.
pub(crate) const INSET: u16 = 1;
/// Blank columns between the words, the bar and the count.
const GAP: u16 = 2;

/// One block's row layout; every row of the list shares it.
pub(crate) struct Geom {
    /// Where a row's words start, after `N ● ` — and a reason line's indent.
    pub text_start: u16,
    /// One past the last column the words may take.
    pub text_end: u16,
    /// The span bar's first column, and its width (0 when dropped).
    pub bar_start: u16,
    pub bar_w: u16,
    /// One past the count column's last column (the pane's margin), and
    /// its width — 0 when no row has a count or the words needed the room.
    pub cost_end: u16,
    pub cost_w: u16,
    /// The richness every row's words fit at (`chatswarmrows::level`).
    pub level: u8,
}

impl Geom {
    /// Columns the words may take.
    pub(crate) fn text_w(&self) -> usize {
        usize::from(self.text_end.saturating_sub(self.text_start))
    }
}

/// The layout with a `cost_w`-wide count column: the count against the
/// margin, the bar a gap to its left, the words a gap short of either.
fn at(s: &SwarmStatus, shown: usize, cols: u16, cost_w: u16) -> Geom {
    let numw = s.tasks.len().to_string().len() as u16;
    let text_start = INSET + numw + 3; // `N ● ` after the inset
    let bar_w = crate::chatswarmspan::bar_cols(cols);
    let cost_end = cols.saturating_sub(1);
    let lane_end = match cost_w {
        0 => cost_end,
        w => cost_end.saturating_sub(w + GAP),
    };
    let bar_start = lane_end.saturating_sub(bar_w);
    let text_end = match bar_w {
        0 => lane_end,
        _ => bar_start.saturating_sub(GAP),
    };
    let mut g = Geom {
        text_start,
        text_end,
        bar_start,
        bar_w,
        cost_end,
        cost_w,
        level: 0,
    };
    g.level = crate::chatswarmrows::level(s, shown, g.text_w());
    g
}

/// The richest layout for the first `shown` rows: with the count column
/// while every row's words still fit whole beside it (and are drawn at all —
/// a row keeps no count it has no title for), else without.
pub(crate) fn geom(s: &SwarmStatus, shown: usize, cols: u16) -> Geom {
    let cost_w = crate::chatswarmcost::col_w(s, shown);
    let with = at(s, shown, cols, cost_w);
    if cost_w == 0 || (with.level == 0 && with.text_w() >= crate::chatswarmrows::MIN_TEXT) {
        with
    } else {
        at(s, shown, cols, 0)
    }
}
