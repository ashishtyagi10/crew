//! A new pane slides out of the pane that opened it.
//!
//! A fresh pane has no drawn rect, so the grid glide had nothing to travel
//! from and set it straight down on its tile: the frame drew itself in
//! (`spawn_timeline`) while the panes beside it glided aside, so the room
//! opened in one motion and the card arrived in another. Now the card starts
//! where the pane that opened it sits — Cmd+D's split comes out of the pane
//! you split — and rides the same springs to its tile.
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

/// The fresh pane to slide in this frame and the rect it starts from: its
/// own tile, moved to the parent's corner. `None` when there is no single
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
            x: from.x,
            y: from.y,
            ..tile
        },
    ))
}

#[cfg(test)]
#[path = "paneentry_tests.rs"]
mod tests;
