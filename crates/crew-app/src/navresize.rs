//! Dragging the sidebar's inner edge to resize it.
//!
//! The nav's width was a number in the Settings form and nowhere else — the
//! one dimension of the layout a user actually wants to nudge, reachable only
//! by opening a pane, typing a figure and saving it. Its edge is a visible
//! boundary sitting right there; now it is also a handle.
//!
//! This is chrome, not layout: the nav is not a pane and the grid does not
//! change shape, it is handed a narrower or wider content rect exactly as it
//! is when the window is resized.
use crate::app::CrewApp;

/// How close to the edge counts as being on it, in logical px each side. A
/// hair wider than it looks: an invisible handle you cannot land on is worse
/// than none.
const GRAB: f32 = 4.0;

/// Width bounds, shared with the Settings form's own clamp so the two paths
/// cannot disagree about what a legal nav width is.
pub(crate) const MIN_W: f32 = 160.0;
pub(crate) const MAX_W: f32 = 320.0;

/// Whether `x` (logical px from the window's left edge) is on the edge of a
/// nav `width` wide.
pub(crate) fn on_edge(x: f32, width: f32) -> bool {
    (x - width).abs() <= GRAB
}

/// The width a drag to `x` asks for, clamped to what the nav may be.
pub(crate) fn width_at(x: f32) -> f32 {
    x.clamp(MIN_W, MAX_W)
}

impl CrewApp {
    /// The cursor's x in logical px, or `None` before the window exists.
    fn cursor_logical_x(&self) -> Option<f32> {
        let (_cw, _ch, _sw, _sh, scale) = self.frame_geometry()?;
        (scale > 0.0).then(|| self.cursor.0 / scale)
    }

    /// Whether the pointer is on the sidebar's resize edge. `false` with the
    /// nav hidden — there is no edge to take hold of.
    pub(crate) fn cursor_on_nav_edge(&self) -> bool {
        self.config.show_nav
            && self
                .cursor_logical_x()
                .is_some_and(|x| on_edge(x, self.config.nav_width))
    }

    /// A left press on the edge takes hold of it. Returns `true` when it did,
    /// so the caller stops before the press reaches a pane.
    pub(crate) fn nav_edge_press(&mut self) -> bool {
        self.nav_drag = self.cursor_on_nav_edge();
        self.nav_drag
    }

    /// Cursor moved with the edge in hand: resize live. Returns `true` when
    /// the frame should be redrawn.
    pub(crate) fn nav_edge_drag(&mut self) -> bool {
        if !self.nav_drag {
            return false;
        }
        let Some(x) = self.cursor_logical_x() else {
            return false;
        };
        let w = width_at(x);
        if (w - self.config.nav_width).abs() < 0.5 {
            return false;
        }
        self.config.nav_width = w;
        true
    }

    /// Release: let go, and persist the width the user settled on. Returns
    /// `true` when this release ended a resize (so it was not a click).
    pub(crate) fn nav_edge_release(&mut self) -> bool {
        if !std::mem::take(&mut self.nav_drag) {
            return false;
        }
        self.config.save();
        true
    }
}

#[cfg(test)]
#[path = "navresize_tests.rs"]
mod tests;
