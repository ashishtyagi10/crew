//! A new pane slides out of the pane that opened it.
//!
//! A fresh pane has no drawn rect, so the grid glide had nothing to travel
//! from and set it straight down on its tile: the frame drew itself in
//! (`spawn_timeline`) while the panes beside it glided aside, so the room
//! opened in one motion and the card arrived in another. Now the card starts
//! a short way back toward the pane that opened it — Cmd+D's split comes out
//! of the side of the pane you split — and rides the same springs to its tile.
//!
//! A short way, not all the way. Only glass gives a card a sheet; on every
//! other page a card is see-through, so one starting ON its parent drew both
//! panes' text through each other for its first frames (shot 2026-11-21).
//! [`REACH`] keeps the direction and the overlap near what the snap had.
//!
//! Position only, never size. A pane's drawn rect is relaid out every frame,
//! and a terminal's grid follows it: a card growing out of nothing would
//! resize a shell that is still printing its first prompt on every frame of
//! the trip. Sliding at its final size, the pty is sized once.
//!
//! One fresh pane only. Panes born together (a restored session, a swarm)
//! have no one parent, and they assemble together on their own clocks.
use crate::layout::Rect;
use crate::pane::Pane;

/// How far back toward the parent's corner the card starts, as a share of
/// the distance between the two.
const REACH: f32 = 0.2;

/// The fresh pane to slide in this frame and the rect it starts from: its
/// own tile, pulled [`REACH`] of the way toward the parent's corner. `None` when there is no single
/// fresh pane or no parent on screen to come out of.
///
/// `placed` is this frame's `(pane, tile)` list; the parent is the pane focus
/// just left for the new one, or the focused pane when the new one opened in
/// the background.
pub(crate) fn entry(
    panes: &[Pane],
    placed: &[(usize, Rect)],
    focused: usize,
    focus_prev: usize,
) -> Option<(usize, Rect)> {
    let fresh = |i: usize| {
        panes
            .get(i)
            .is_some_and(|p| p.rect.w <= 0.0 || p.rect.h <= 0.0)
    };
    let mut born = placed.iter().filter(|(i, _)| fresh(*i));
    let &(idx, tile) = born.next()?;
    if born.next().is_some() {
        return None;
    }
    let parent = if focused == idx { focus_prev } else { focused };
    let on_screen = parent != idx && placed.iter().any(|(i, _)| *i == parent);
    let from = panes.get(parent).filter(|_| on_screen).map(|p| p.rect)?;
    if from.w <= 0.0 || from.h <= 0.0 {
        return None;
    }
    Some((
        idx,
        Rect {
            x: tile.x + (from.x - tile.x) * REACH,
            y: tile.y + (from.y - tile.y) * REACH,
            ..tile
        },
    ))
}

#[cfg(test)]
#[path = "paneentry_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "entryshot_tests.rs"]
mod shots;
