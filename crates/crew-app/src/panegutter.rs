//! Dragging the scroll thumb on a pane's right border.
//!
//! The thumb ([`crate::panescroll`]) says where you are in the buffer. Once
//! something on screen says where you are, the next thing anyone tries is
//! moving it — and a proportional gutter is the only control in a terminal
//! that can cross ten thousand lines in one gesture, which neither the wheel
//! nor a page key can.
//!
//! The gutter is live only while the thumb is drawn, which is only while the
//! pane is scrolled back: at the live bottom there is no gutter to grab,
//! because there is nothing behind it to reach.
use crate::app::CrewApp;
use crate::pane::PaneContent;

/// Half-width of the grab region around the right border, in cells. A one-cell
/// target is not a target.
const GRAB_CELLS: f32 = 1.0;

/// Where in a card's gutter a pointer at `y` px is, as `0.0..=1.0` from the
/// top of the interior. `None` when the card has no interior to speak of.
pub(crate) fn gutter_frac(rect_y: f32, rect_h: f32, ch: f32, y: f32) -> Option<f32> {
    let inner_h = rect_h - 2.0 * ch;
    if inner_h <= 0.0 {
        return None;
    }
    Some(((y - rect_y - ch) / inner_h).clamp(0.0, 1.0))
}

impl CrewApp {
    /// Whether the pointer is on a live gutter — read by [`crate::pointer`]
    /// to shape the cursor.
    pub(crate) fn cursor_on_gutter(&self) -> bool {
        self.gutter_at_cursor().is_some()
    }

    /// The pane whose right-border gutter the cursor is on, with the rect it
    /// was found in. Only a scrolled-back terminal has one.
    fn gutter_at_cursor(&self) -> Option<(usize, crate::layout::Rect)> {
        let (cw, _ch, _sw, _sh, _scale) = self.frame_geometry()?;
        let (_content, placed) = self.placed_grid()?;
        placed.full.into_iter().find(|&(idx, r)| {
            let on_edge = (self.cursor.0 - (r.x + r.w)).abs() <= GRAB_CELLS * cw;
            let in_rows = self.cursor.1 >= r.y && self.cursor.1 <= r.y + r.h;
            on_edge && in_rows && self.pane_scrolled(idx)
        })
    }

    /// Whether pane `idx` is a terminal currently scrolled back — the only
    /// state in which a gutter is drawn, and so the only one it can be
    /// grabbed in.
    fn pane_scrolled(&self, idx: usize) -> bool {
        matches!(self.panes.get(idx).map(|p| &p.content),
            Some(PaneContent::Terminal(t)) if t.pty.display_offset() > 0)
    }

    /// A left press on a gutter takes hold of it and jumps there at once — a
    /// scrollbar that only moves on the second event feels broken. Returns
    /// `true` when the press was claimed.
    pub(crate) fn gutter_press(&mut self) -> bool {
        let Some((idx, _)) = self.gutter_at_cursor() else {
            self.gutter_drag = None;
            return false;
        };
        self.gutter_drag = Some(idx);
        self.gutter_seek();
        true
    }

    /// Cursor moved with a gutter in hand: scroll to where it points.
    /// Returns `true` when the frame should be redrawn.
    pub(crate) fn gutter_drag_move(&mut self) -> bool {
        self.gutter_drag.is_some() && self.gutter_seek()
    }

    /// Let go. `true` when this release ended a gutter drag.
    pub(crate) fn gutter_release(&mut self) -> bool {
        self.gutter_drag.take().is_some()
    }

    /// Scroll the carried pane to the line the pointer is over.
    fn gutter_seek(&mut self) -> bool {
        let Some(idx) = self.gutter_drag else {
            return false;
        };
        let Some((_cw, ch, _sw, _sh, _scale)) = self.frame_geometry() else {
            return false;
        };
        let Some(rect) = self
            .pane_hit_rects()
            .into_iter()
            .find(|&(i, _)| i == idx)
            .map(|(_, r)| r)
        else {
            return false;
        };
        let Some(frac) = gutter_frac(rect.y, rect.h, ch, self.cursor.1) else {
            return false;
        };
        let rows = self.panes.get(idx).map_or(0, |p| p.grid.rows) as usize;
        let Some(PaneContent::Terminal(t)) = self.panes.get_mut(idx).map(|p| &mut p.content) else {
            return false;
        };
        let want = crate::panescroll::offset_at(t.pty.scrollable_lines(), rows, frac);
        let delta = want as i64 - t.pty.display_offset() as i64;
        if delta == 0 {
            return false;
        }
        t.pty
            .scroll(delta.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32);
        true
    }
}

#[cfg(test)]
#[path = "panegutter_tests.rs"]
mod tests;
