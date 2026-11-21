//! Scrolling that travels instead of jumping.
//!
//! A wheel notch, Shift+PageUp/PageDown and Shift+Home/End used to move a
//! pane's scrollback in one frame: three rows, a page, or the whole buffer
//! gone between two frames, so the eye had nothing to follow and lost its
//! place. Now the offset rides a [`Spring`] — the grid glide's own primitive —
//! and the pane is handed whole rows as the spring passes them. A second
//! notch mid-flight adds to where it is going and keeps the speed it has,
//! which is what makes a run of notches one smooth motion, not a stutter.
//!
//! Whole rows, on purpose: every reader of the offset (selection, find,
//! marks, pictures, hit-testing) keeps meaning what it meant. A pixel offset
//! would need cell backgrounds clipped to the card and a row from beyond the
//! viewport, neither of which the renderer has.
//!
//! Bounded: a critically damped spring converges and the snap ends it, so
//! [`crate::paneview::pane_animating`] goes quiet. At Motion off nothing is
//! queued; the scroll is the jump it always was. The trackpad never comes
//! here: it brings its own momentum, and a spring on top is only lag.
use crate::pane::{Pane, PaneContent};
use crate::spring::Spring;

/// Stiffness, in rad/s: a notch (3 rows) lands in ~150 ms, a page of 40 in
/// ~300 ms — fast enough to never feel like waiting.
const OMEGA: f32 = 24.0;

/// One pane's scroll in flight. Units are rows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ScrollGlide {
    /// How far the glide has travelled, and how fast it is going.
    pos: Spring,
    /// How far it was asked to travel (positive = up, into history).
    total: f32,
    /// Whole rows already handed to the pane.
    applied: i32,
    /// A terminal's offset after the last step — how a snap to the live
    /// bottom made by someone else (typing, a program's output) is noticed.
    mark: Option<usize>,
}

impl ScrollGlide {
    /// Ask for `lines` more, on top of whatever is still in flight.
    pub(crate) fn push(&mut self, lines: i32) {
        self.total += lines as f32;
    }

    /// Whether rows are still owed to the pane. Not just `total`: a reverse
    /// notch can cancel the trip to zero while rows are already handed out,
    /// and those have to be travelled back.
    pub(crate) fn moving(&self) -> bool {
        self.total != 0.0 || self.applied != 0 || self.pos != Spring::default()
    }

    /// Drop whatever is in flight.
    pub(crate) fn stop(&mut self) {
        *self = Self::default();
    }

    /// Advance `dt_ms`; the whole rows to scroll this frame.
    pub(crate) fn step(&mut self, dt_ms: u64) -> i32 {
        if !self.moving() {
            return 0;
        }
        self.pos.step(self.total, dt_ms as f32, OMEGA);
        let settled = self.pos.settled(self.total);
        let want = match settled {
            true => self.total.round() as i32,
            false => self.pos.pos.round() as i32,
        };
        let delta = want - self.applied;
        self.applied = want;
        if settled {
            self.stop();
        }
        delta
    }
}

/// Scroll `pane` by `lines`: queued on its glide while motion is on, at once
/// when it is off.
pub(crate) fn start(pane: &mut Pane, lines: i32) {
    if lines == 0 {
        return;
    }
    if crate::motion::level() == crate::motion::MotionLevel::Off {
        crate::scroll::scroll_pane(pane, lines);
        return;
    }
    if !pane.glide.scroll.moving() {
        pane.glide.scroll.mark = term_offset(pane);
    }
    pane.glide.scroll.push(lines);
}

/// Shift+Home/End. A terminal knows exactly how far its ends are, so the
/// trip is measured and glides; any other pane is asked to clamp a huge
/// delta, which only makes sense as the jump.
pub(crate) fn start_end(pane: &mut Pane, to_top: bool) {
    pane.glide.scroll.stop();
    let lines = match &pane.content {
        PaneContent::Terminal(t) => {
            let off = t.pty.display_offset() as i32;
            match to_top {
                true => t.pty.history_lines() as i32 - off,
                false => -off,
            }
        }
        _ => {
            let far = if to_top { i32::MAX / 2 } else { i32::MIN / 2 };
            return crate::scroll::scroll_pane(pane, far);
        }
    };
    start(pane, lines);
}

/// One frame of every pane's glide; whether any is still moving.
pub(crate) fn step_all(panes: &mut [Pane], dt_ms: u64) -> bool {
    for p in panes.iter_mut().filter(|p| p.glide.scroll.moving()) {
        let before = term_offset(p);
        // The terminal went back to its live bottom under us — the user
        // typed, or a program asked. Following through would scroll away
        // from what they are now looking at.
        if before == Some(0) && p.glide.scroll.mark.is_some_and(|m| m != 0) {
            p.glide.scroll.stop();
            continue;
        }
        let d = p.glide.scroll.step(dt_ms);
        if d != 0 {
            crate::scroll::scroll_pane(p, d);
        }
        let after = term_offset(p);
        // An end of the buffer: nothing left to travel through.
        if d != 0 && after == before && before.is_some() {
            p.glide.scroll.stop();
        }
        if p.glide.scroll.moving() {
            p.glide.scroll.mark = after;
        }
    }
    panes.iter().any(|p| p.glide.scroll.moving())
}

fn term_offset(p: &Pane) -> Option<usize> {
    match &p.content {
        PaneContent::Terminal(t) => Some(t.pty.display_offset()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "scrollglide_tests.rs"]
mod tests;
