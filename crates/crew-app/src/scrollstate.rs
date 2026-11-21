//! The state of one pane's scroll glide — the spring, the rows it has handed
//! over, and the remainder drawn between them. The pane side (starting,
//! stepping, ends of the buffer) is [`crate::scrollglide`].
use crate::spring::Spring;

/// Stiffness, in rad/s: a notch (3 rows) lands in ~150 ms, a page of 40 in
/// ~300 ms — fast enough to never feel like waiting.
pub(crate) const OMEGA: f32 = 24.0;

/// Stiffness for the trackpad: three frames to follow a row, about.
pub(crate) const TRACK_OMEGA: f32 = 60.0;

/// Spring units per row. [`Spring`] settles within half a unit; in rows that
/// was half a row, and a pixel-drawn scroll snapping the last half row was
/// the jump it exists to remove. Half a 16th of a row is about a pixel at
/// the largest cell heights; finer only drags out an invisible tail.
const UNIT: f32 = 16.0;

/// One pane's scroll in flight. Units are rows, [`UNIT`]s inside the spring.
#[derive(Clone, Debug, Default)]
pub(crate) struct ScrollGlide {
    /// How far the glide has travelled, and how fast it is going.
    pos: Spring,
    /// The stiffness this trip runs at; 0 is [`OMEGA`].
    omega: f32,
    /// A terminal's line just under its viewport, for the frame being drawn
    /// while it is between two rows.
    pub(crate) edge: Option<Vec<crew_term::RenderCell>>,
    /// How far it was asked to travel (positive = up, into history).
    total: f32,
    /// Whole rows already handed to the pane.
    applied: i32,
    /// A terminal's offset after the last step — how a snap to the live
    /// bottom made by someone else (typing, a program's output) is noticed.
    pub(crate) mark: Option<usize>,
    /// Drawn between rows (a terminal): the pane is handed the row at or
    /// above the spring and the remainder is drawn. Every other pane is
    /// handed the NEAREST row — it can only draw whole ones, and taking the
    /// row ahead would make one direction lag the other.
    pub(crate) pixel: bool,
}

impl ScrollGlide {
    /// Ask for `lines` more, on top of whatever is still in flight.
    pub(crate) fn push(&mut self, lines: i32) {
        self.total += lines as f32 * UNIT;
    }

    /// [`Self::push`] at stiffness `omega` (the trackpad's).
    pub(crate) fn push_at(&mut self, lines: i32, omega: f32) {
        self.omega = omega;
        self.push(lines);
    }

    /// Where the drawing is relative to the rows handed over, in rows:
    /// `(-1, 0]`, the content drawn that far UP. Zero at rest.
    pub(crate) fn residual(&self) -> f32 {
        match self.pixel && self.moving() {
            true => (self.pos.pos / UNIT - self.applied as f32).clamp(-0.999, 0.0),
            false => 0.0,
        }
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
        let omega = if self.omega > 0.0 { self.omega } else { OMEGA };
        self.pos.step(self.total, dt_ms as f32, omega);
        let settled = self.pos.settled(self.total);
        // The row at or above the spring when the remainder is drawn (by
        // sliding up, see `scrollglide`), else the nearest; exact once settled.
        let at = self.pos.pos / UNIT;
        let want = match (settled, self.pixel) {
            (true, _) => (self.total / UNIT).round() as i32,
            (false, true) => at.ceil() as i32,
            (false, false) => at.round() as i32,
        };
        let delta = want - self.applied;
        self.applied = want;
        if settled {
            self.stop();
        }
        delta
    }

    /// Whole rows handed over so far.
    #[cfg(test)]
    pub(crate) fn applied(&self) -> i32 {
        self.applied
    }
}

#[cfg(test)]
#[path = "scrollstate_tests.rs"]
mod tests;
