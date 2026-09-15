//! The rail's foot: the nav's dashboard, at five columns.
//!
//! The rail shipped as the pane list and nothing else, and a pane list is two
//! or three rows — on a full-height window that left forty rows of empty
//! column standing exactly where the dashboard used to be. A collapsed nav
//! that says less than the space it takes is worse than no nav: it costs the
//! same edge of screen and spends it on nothing.
//!
//! So the foot carries the readings that survive five columns. The clock in
//! full, and the sky over the place the user named. The three system readings
//! as drawn capsules, in the gauges' own tier colours — the shape reads
//! without digits, which is the only way a load fits here at all. The two net
//! rates in a rail-sized unit. What git has to say about the tree. Everything
//! else the nav knows stays one click away, which is the rail's contract.
//!
//! It is anchored to the BOTTOM of the column, not stacked under the panes: a
//! reading that moves every time a pane opens is a reading you have to find
//! again, and the pane list is the one section here that changes height.
use crew_render::{CellView, Paint};

use crate::git::GitInfo;
use crate::navrailrow as row;
use crate::stats::Stats;

/// What the foot draws this frame. Pure data — the readings are taken once,
/// by the caller, off the same sampler and watcher the open nav's own
/// sections read, so the two can never disagree about the machine.
pub(crate) struct Foot {
    /// Wall clock, `HH:MM`: at five columns the seconds are what give.
    pub time: String,
    /// The weather glyph and this hour's temperature, when a place is set.
    pub sky: Option<String>,
    pub stats: Stats,
    pub git: Option<GitInfo>,
}

/// One section of the foot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Group {
    Time,
    Sky,
    Sys,
    Net,
    Git,
}

impl Group {
    /// Rows it occupies, its own gap NOT included.
    fn rows(self, f: &Foot) -> u16 {
        match self {
            Group::Time | Group::Sky => 1,
            Group::Sys => 3,
            Group::Net => 2,
            // The `↑↓` row exists only when there is something to say.
            Group::Git => 1 + u16::from(f.git.as_ref().is_some_and(|i| i.ahead + i.behind > 0)),
        }
    }

    /// The blank row before it. Every group gets one but the first — and the
    /// sky, which rides directly under the clock exactly as the weather strip
    /// rides in the clock's own gap row on the open nav.
    fn gap_before(self, first: bool) -> u16 {
        u16::from(!first && self != Group::Sky)
    }

    /// Whether it has anything to draw at all: no repo, no GIT rows.
    fn has(self, f: &Foot) -> bool {
        match self {
            Group::Sky => f.sky.is_some(),
            Group::Git => f.git.is_some(),
            _ => true,
        }
    }
}

/// Top to bottom — the order the open nav stacks the same sections in (the
/// weather rides with the clock there too), so opening the rail moves nothing
/// sideways in the reader's head.
const ORDER: [Group; 5] = [Group::Time, Group::Sky, Group::Sys, Group::Net, Group::Git];

/// What goes first when the column is short, in order. The rates are the
/// reading you would have opened the nav for anyway; the clock is the one you
/// would not, so the clock is what a rail with one row left still shows.
const DROP: [Group; 4] = [Group::Sky, Group::Net, Group::Git, Group::Sys];

/// Rows a stack occupies, the blank rows between its groups included.
fn height(shown: &[Group], f: &Foot) -> u16 {
    shown
        .iter()
        .enumerate()
        .map(|(i, g)| g.rows(f) + g.gap_before(i == 0))
        .sum()
}

/// Where the foot sits in a rail of `rows` content rows whose pane list took
/// `used` of them: its top row, and the groups that fit, in draw order.
///
/// The pane list is served first — it is the rail's navigation control, and a
/// row pushed off it is a pane you cannot click — and one blank row always
/// stands between the last pane and the first reading.
pub(crate) fn plan(f: &Foot, rows: u16, used: u16) -> (u16, Vec<Group>) {
    let mut shown: Vec<Group> = ORDER.into_iter().filter(|g| g.has(f)).collect();
    let room = rows.saturating_sub(used.saturating_add(1));
    for drop in DROP {
        if height(&shown, f) <= room {
            break;
        }
        shown.retain(|g| *g != drop);
    }
    if height(&shown, f) > room {
        shown.clear();
    }
    (rows.saturating_sub(height(&shown, f)), shown)
}

/// The foot's cells and the sub-cell paint its meters are drawn on, in the
/// card interior's own coordinates. `aspect` is `cell_h / cell_w`, so a
/// capsule keeps its proportions at any font size.
pub(crate) fn foot(
    f: &Foot,
    cols: u16,
    rows: u16,
    used: u16,
    aspect: f32,
) -> (Vec<CellView>, Vec<Paint>) {
    let (top, shown) = plan(f, rows, used);
    let (mut cells, mut paint) = (Vec::new(), Vec::new());
    let t = crew_theme::theme();
    let mut row = top;
    for (i, g) in shown.iter().enumerate() {
        row += g.gap_before(i == 0);
        match g {
            Group::Time => crate::navtext::put_at(&mut cells, &f.time, 0, row, cols, t.ink),
            Group::Sky => {
                let sky = f.sky.clone().unwrap_or_default();
                crate::navtext::put_at(&mut cells, &sky, 0, row, cols, t.text_muted)
            }
            Group::Sys => row::sys(&mut cells, &mut paint, f.stats, row, cols, aspect),
            Group::Net => row::net(&mut cells, f.stats.net_rx, f.stats.net_tx, row, cols),
            Group::Git => row::git(&mut cells, f.git.as_ref(), row, cols),
        }
        row += g.rows(f);
    }
    (cells, paint)
}

#[cfg(test)]
#[path = "navrailfoot_tests.rs"]
mod tests;
