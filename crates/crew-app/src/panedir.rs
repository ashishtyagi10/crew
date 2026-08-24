//! Spatial pane navigation: pick the neighbour that lies in a *direction*.
//!
//! Panes auto-tile into a near-square grid, so `Cmd+[` / `Cmd+]` — which step
//! through panes in index order — walk a path that has nothing to do with what
//! the eye sees: in a 2×2 grid, index 1 sits *below* index 0, not beside it.
//! This module answers the question the arrow keys actually ask ("which card
//! is to my left?") from the same rects the mouse hit-tests against, so
//! keyboard focus and the pointer agree about where a tile is.
use crate::layout::Rect;

/// One of the four arrow directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dir {
    Left,
    Right,
    Up,
    Down,
}

impl Dir {
    /// `true` when this direction travels along the x axis.
    fn horizontal(self) -> bool {
        matches!(self, Dir::Left | Dir::Right)
    }

    /// `true` when moving in this direction *decreases* the coordinate.
    fn negative(self) -> bool {
        matches!(self, Dir::Left | Dir::Up)
    }
}

/// Centre of `r` on the travel axis and on the axis across it, as
/// `(along, across)` — so the scoring below can be written once instead of
/// four times.
fn centre(r: Rect, dir: Dir) -> (f32, f32) {
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    if dir.horizontal() {
        (cx, cy)
    } else {
        (cy, cx)
    }
}

/// The pane in `rects` to focus when moving `dir` from the pane at index
/// `from`, or `None` when nothing lies that way (the edge of the grid — focus
/// stays put rather than wrapping, because a wrap in a spatial gesture reads
/// as the grid jumping).
///
/// Candidates must sit strictly beyond the current pane on the travel axis;
/// among them the nearest along that axis wins, and ties — the common case,
/// since a tiled column shares one x — break toward the pane whose centre is
/// closest across it. `rects` is `(pane_index, rect)` exactly as
/// [`crate::framegeo`] hands it out, so minimized strip thumbnails are
/// reachable too: they simply sit below every full tile.
pub(crate) fn step(rects: &[(usize, Rect)], from: usize, dir: Dir) -> Option<usize> {
    let cur = rects.iter().find(|&&(i, _)| i == from)?.1;
    let (a0, c0) = centre(cur, dir);
    // Distances are measured in the direction of travel, so a single
    // comparison covers both signs.
    let sign = if dir.negative() { -1.0 } else { 1.0 };
    rects
        .iter()
        .filter(|&&(i, _)| i != from)
        .filter_map(|&(i, r)| {
            let (a, c) = centre(r, dir);
            let along = (a - a0) * sign;
            // A neighbour must be a full cell-ish beyond us on the axis:
            // floating-point tile edges make an exact `> 0.0` fragile.
            (along > 1.0).then_some((along, (c - c0).abs(), i))
        })
        .min_by(|x, y| {
            x.0.total_cmp(&y.0)
                .then(x.1.total_cmp(&y.1))
                .then(x.2.cmp(&y.2))
        })
        .map(|(_, _, i)| i)
}

impl crate::app::CrewApp {
    /// `Cmd+Arrow`: move focus to the card that lies in `dir`. Silent at the
    /// edge of the grid — the gesture is spatial, so there is nothing to say
    /// when there is nothing over there.
    pub(crate) fn focus_direction(&mut self, dir: Dir) {
        // Zoomed, one pane fills the content area and there is no geometry to
        // navigate; fall back to the index step so the chord still moves.
        if self.zoomed {
            self.focus_visible_step(if dir.negative() { -1 } else { 1 });
            self.input.focused = false;
            return;
        }
        if let Some(i) = step(&self.pane_hit_rects(), self.focused, dir) {
            self.focused = i;
            self.input.focused = false;
        }
    }

    /// `Cmd+Shift+Arrow`: swap the focused pane with the card in `dir`, so a
    /// tile can be dragged around the grid without counting index positions.
    /// Restricted to the full tiles: the minimized strip's membership is the
    /// LRU's to decide, not a keystroke's.
    pub(crate) fn move_direction(&mut self, dir: Dir) {
        let Some((_, placed)) = self.placed_grid() else {
            return;
        };
        if let Some(j) = step(&placed.full, self.focused, dir) {
            self.panes.swap(self.focused, j);
            self.focused = j;
            self.input.focused = false;
        }
    }
}

#[cfg(test)]
#[path = "panedir_tests.rs"]
mod tests;
