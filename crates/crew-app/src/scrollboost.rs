//! Scrolling further when you mean to go further.
//!
//! A wheel tick is a fixed number of lines, so crossing ten thousand lines of
//! a build log means the same gesture two hundred times. Every editor and
//! browser answers this by scaling with the *speed* of the gesture: flicking
//! moves pages, nudging moves lines, and the two are the same wheel.
//!
//! The multiplier is a function of how quickly ticks are arriving, not of how
//! many have arrived, so it decays the moment you stop — a scroll you resume
//! after a pause starts slow again, which is what "start slow" has to mean for
//! it to be useful for reading.
/// Ticks closer together than this are one gesture.
const GAP_MS: u64 = 120;

/// The most a fast gesture is multiplied by. Chosen against the thing being
/// crossed: at three lines a tick, six is a page a flick — fast enough to
/// cross a long log, slow enough that a single flick never overshoots the
/// whole scrollback.
const MAX: f32 = 6.0;

/// How much each successive tick in one gesture adds to the multiplier.
const STEP: f32 = 0.6;

/// Wheel-gesture speed, kept between ticks.
#[derive(Default)]
pub(crate) struct Boost {
    last_ms: u64,
    factor: f32,
}

impl Boost {
    /// The multiplier for a tick arriving at `now_ms`. A tick that follows
    /// closely builds on the last one; a pause resets to one.
    pub(crate) fn factor(&mut self, now_ms: u64) -> f32 {
        let gap = now_ms.saturating_sub(self.last_ms);
        self.last_ms = now_ms;
        self.factor = match gap <= GAP_MS && self.factor > 0.0 {
            true => (self.factor + STEP).min(MAX),
            false => 1.0,
        };
        self.factor
    }

    /// Apply the multiplier to a line count, keeping its direction and never
    /// rounding a real tick down to nothing.
    pub(crate) fn apply(&mut self, lines: i32, now_ms: u64) -> i32 {
        if lines == 0 {
            return 0;
        }
        let scaled = (lines as f32 * self.factor(now_ms)).round() as i32;
        match scaled == 0 {
            true => lines.signum(),
            false => scaled,
        }
    }
}

#[cfg(test)]
#[path = "scrollboost_tests.rs"]
mod tests;
