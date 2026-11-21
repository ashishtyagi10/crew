//! A tab comes in from the side of the list it was on.
//!
//! Switching tabs already re-assembled the incoming cards (`switch_grid`
//! re-stamps `born_ms`), but every switch looked the same: Cmd+Shift+] and
//! Cmd+Shift+[ drew the identical arrival, so nothing said which way through
//! the tabs you had gone. The nav lists tabs top to bottom, so a tab further
//! down the list rises in from below and one further up drops in from above —
//! the same direction the accent moves in the nav.
//!
//! A short travel on the grid's own springs, not a full-height slide: nothing
//! clips panes to the content area, so a card coming up from below passes
//! over the input bar for its first frames (going up, the window edge cuts
//! it). 8% keeps that to a few rows, and the assemble carries the rest.
//! Position only, like every entrance (`paneentry`): no pty is resized on the
//! way in. Motion off snaps, as the glide always does.
use crate::layout::Rect;
use crate::tabs::Tabs;

/// How far a card travels in, as a share of its own height.
const TRAVEL: f32 = 0.08;

/// +1 when `to` is further down the nav than the tab on screen, -1 when it
/// is further up, 0 when either is unknown (a tab that just went away).
pub(crate) fn direction(tabs: &Tabs, to: u32) -> i32 {
    match (tabs.pos(tabs.active), tabs.pos(to)) {
        (Some(from), Some(to)) => (to as i32 - from as i32).signum(),
        _ => 0,
    }
}

/// Where a card bound for `tile` starts when its tab arrives going `dir`.
pub(crate) fn start(tile: Rect, dir: i32) -> Rect {
    Rect {
        y: tile.y + dir.signum() as f32 * tile.h * TRAVEL,
        ..tile
    }
}

#[cfg(test)]
#[path = "tabentry_tests.rs"]
mod tests;
