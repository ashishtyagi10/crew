//! On a soft nav, the tab you are on stands up out of the panel.
//!
//! The soft nav is one raised panel with every tab's card sunk into it as a
//! well ([`crate::navreliefscene`]). Once the strokes were blanked for that,
//! the only thing left saying which tab was current was the colour of its
//! number and name: every well sat at the same depth (the open list of
//! 2026-11-14: "active tab well no deeper than the rest"). A deeper well
//! was tried first (-0.95 against -0.65) and could not be told apart in the
//! shots: past its rest the renderer deepens a well's inner shadow, not its
//! reach. A segmented control says "selected" the other way up — iOS draws
//! the chosen segment as a raised card on a sunk track — so the active tab
//! is raised as a key is (`reliefbutton`), the others stay sunk.
//! Read off the cells before their strokes are blanked: the active card's
//! frame is drawn in the accent (`tabnav`), a waiting one's in the bell.
use crew_render::CellView;

use crate::reliefwell::{Box4, DEPTH};

/// The active tab's lift: a key's ([`crate::reliefbutton::RAISE`]).
pub(crate) const ACTIVE: f32 = crate::reliefbutton::RAISE;

/// The box among `tabs` whose top-left corner is drawn in `accent`.
pub(crate) fn active(cells: &[CellView], tabs: &[Box4], accent: (u8, u8, u8)) -> Option<Box4> {
    tabs.iter().copied().find(|b| {
        cells
            .iter()
            .any(|c| c.row == b.r0 && c.col == b.c0 && c.c == '\u{256d}' && c.fg == accent)
    })
}

/// How far a tab's card stands from the panel: up for the active one.
pub(crate) fn depth(active: bool) -> f32 {
    if active {
        ACTIVE
    } else {
        DEPTH
    }
}

#[cfg(test)]
#[path = "navreliefactive_tests.rs"]
mod tests;
