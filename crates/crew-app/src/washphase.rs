//! The clock behind the gradient's hue breath: while a pane works, the poles
//! that light every focused card's ring lean a little off the theme's own
//! colour and back ([`WashPhase::hue_deg`], published through crew-theme's
//! `poleshift`). It was also the clock of the page's wash — the backdrop that
//! drifted behind the panes — until every theme went see-through (2026-10-09)
//! and the desktop became the only background.
//!
//! The phase is advanced from ELAPSED TIME BETWEEN DRAWN FRAMES rather than
//! read off the wall clock, so it never jumps: accumulating deltas means the
//! breath is continuous across every pause. It rides frames that activity was
//! already drawing, so it costs nothing; its **pace**, in ms per revolution or
//! `None` to hold, lives in [`crate::washgate`]. A change of pace is a glide,
//! not a jump: the clock's speed follows it through a flywheel
//! ([`SPIN_UP_MS`]).
use crate::motion::MotionLevel;

/// Longest delta a single frame may contribute, in ms. A frame gap longer
/// than this is a stall (a slow build, a laptop lid, a blocking read on the
/// winit thread), and paying it back in one step would show up as a lurch.
const MAX_STEP_MS: u64 = 250;

/// How quickly the clocks' SPEED follows a change of pace, in ms: the time
/// constant of the flywheel. A pane that starts working quickens the glow's
/// beat over a couple of seconds rather than lurching to twice
/// the speed in one frame, and when the work ends it coasts back down
/// over several — slower down than up, the way a heavy wheel spins up under
/// power and then coasts.
const SPIN_UP_MS: f32 = 900.0;
const SPIN_DOWN_MS: f32 = 2_800.0;

/// How much slower the gradient's HUE breathes than the phase turns: at the
/// themes' 6 s `drift_ms`, a 24-second breath while a pane works — slow
/// enough to be a mood, fast enough to see it change.
pub(crate) const HUE_MULT: u64 = 2;

#[cfg(test)]
use crate::washgate::pace;

/// The phase and the gradient's hue breath, both in turns.
#[derive(Default)]
pub(crate) struct WashPhase {
    phase: f32,
    /// Where the hue breath is in its cycle, in turns — [`HUE_MULT`] times
    /// slower than `phase` and read through [`Self::hue_deg`].
    hue: f32,
    /// How fast the phase is turning, in turns per ms — eased toward the
    /// pace's rate (see [`SPIN_UP_MS`]). `0.0` until the first frame, which
    /// starts at its pace outright.
    rate: f32,
    /// When the last DRIFTING frame was stamped. Cleared whenever the wash
    /// holds, so the first frame after a hold contributes nothing and the
    /// still time in between is never paid back.
    last_ms: Option<u64>,
}

impl WashPhase {
    /// This frame's phase. Advances by the time since the previous drawn
    /// frame at `pace` ms per revolution ([`pace`] decides which) — reached
    /// through the flywheel, so a change of pace is a glide — holds on
    /// `None` — and holds at Motion off, which is a genuine off and not a slow
    /// setting. The motion level is passed in rather than read from the global
    /// so the clock is a pure function of its inputs.
    pub(crate) fn advance(&mut self, now_ms: u64, pace: Option<u64>, motion: MotionLevel) -> f32 {
        let Some(pace) = pace.filter(|&p| p > 0 && motion != MotionLevel::Off) else {
            self.last_ms = None;
            return self.phase;
        };
        let dt = self
            .last_ms
            .map_or(0, |last| now_ms.saturating_sub(last).min(MAX_STEP_MS));
        self.last_ms = Some(now_ms);
        let (dt, target) = (dt as f32, 1.0 / pace as f32);
        self.rate = match self.rate {
            r if r == 0.0 => target,
            r => {
                let tau = if target > r { SPIN_UP_MS } else { SPIN_DOWN_MS };
                r + (target - r) * (1.0 - (-dt / tau).exp())
            }
        };
        let step = self.rate * dt;
        self.phase = (self.phase + step).fract();
        self.hue = (self.hue + step / HUE_MULT as f32).fract();
        self.phase
    }

    /// This frame's hue offset in degrees: `span` either side of the theme's
    /// own colour, as a SINE of the hue clock rather than a rotation.
    ///
    /// A sine is what makes it a breath — the poles lean, pass back through
    /// the palette's exact colour, and lean the other way — where a monotonic
    /// rotation would eventually walk every theme through every hue and stop
    /// being that theme. Exactly `0.0` at rest (`sin 0`), so a process that
    /// has never drifted wears the theme's own bytes.
    pub(crate) fn hue_deg(&self, span: f32) -> f32 {
        span * (std::f32::consts::TAU * self.hue).sin()
    }
}

#[cfg(test)]
#[path = "washphase_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "washlive_tests.rs"]
mod glide_tests;
