//! The clock behind the modern backdrop's gradient wash: where the two pools
//! of pole light sit on their orbit this frame.
//!
//! The wash itself is drawn by the background pass (see crew-render's
//! `ModernPaper`); all that lives here are the two numbers it moves by — the
//! orbit, which also breathes the pools, turns the dot lattice's tint and
//! turns its glint's spiral arms, and the slower wander/hue clock, which also
//! winds the page's whirlpool — and the rule for when
//! they are allowed to move: their **pace**, in ms per revolution, or `None`
//! to hold.
//!
//! The phase is advanced from ELAPSED TIME BETWEEN DRAWN FRAMES rather than
//! read off the wall clock, so it never jumps: a wall-clock phase would
//! teleport the pools across the page after a quiet minute, while accumulating
//! deltas means the motion is continuous across every pause.
//!
//! ## Two paces
//!
//! **Busy** is the original one: a revolution per the theme's `drift_ms` (six
//! seconds), riding frames that activity was already drawing, so it cost
//! nothing.
//!
//! **Ambient** is the one that makes a quiet window feel alive rather than
//! frozen. It is [`AMBIENT_MULT`] times slower, and it is the only motion in
//! crew that asks for frames nothing else needed — which is why it is fenced:
//! the user's `ambient_drift` setting, Motion not off, a theme that has a wash
//! at all, and crew holding the OS focus. A window you are not looking at
//! repaints for nobody.
//!
//! Turning ambient off restores the old behaviour exactly, including the
//! static-frame determinism the CRT trace and the modern ring keep — which is
//! also what every headless shot test relies on.
use crate::motion::MotionLevel;

/// Longest delta a single frame may contribute, in ms. A frame gap longer
/// than this is a stall (a slow build, a laptop lid, a blocking read on the
/// winit thread), and paying it back in one step would show up as a lurch.
const MAX_STEP_MS: u64 = 250;

/// How much slower the idle drift is than the busy one. At the themes'
/// 6 s `drift_ms` this is a revolution every 24 seconds — 15° of orbit per
/// second, a pool breath and a bloom of the glint's arms every 12 — which reads
/// as a room whose light is plainly alive without anything asking for your
/// attention. Busy motion is still the faster signal. (It was 15, a
/// 90-second revolution, then 10; at both, the user read the page as not
/// animating at all.)
pub(crate) const AMBIENT_MULT: u64 = 4;

/// How much slower the gradient's HUE breathes than the pools orbit.
///
/// The two clocks share one accumulator ([`WashPhase`]) because they share
/// one set of fences and one supply of frames — the hue costs nothing that
/// the orbit was not already paying for. They run at different rates because
/// a colour that changed in lockstep with the position it is drawn at reads
/// as one effect with a stutter; two to one, the colour comes back to the
/// theme's own on every other turn of the pools, never in step with them. At
/// the themes' 6 s `drift_ms` that is a 12-second breath while a pane works
/// and a 48-second one while the room is quiet — slow enough to be a mood,
/// fast enough to see it change. (It was four to one, a four-minute idle
/// breath nobody could watch happen.) The pools' wander rides the same clock
/// (see [`WashPhase::wander`]); NOT three, whose third harmonic would put the
/// wander's lean in lockstep with the orbit.
pub(crate) const HUE_MULT: u64 = 2;

/// Ms per revolution this frame, or `None` to hold where it is.
///
/// `busy` wins over `ambient`: a working pane's wash keeps its own faster
/// pace, so the two never fight over one phase, and stepping between them is
/// continuous because both accumulate onto the same number.
pub(crate) fn pace(drift_ms: u64, busy: bool, ambient: bool) -> Option<u64> {
    if drift_ms == 0 {
        return None;
    }
    match (busy, ambient) {
        (true, _) => Some(drift_ms),
        (false, true) => Some(drift_ms.saturating_mul(AMBIENT_MULT)),
        (false, false) => None,
    }
}

/// The wash's orbital position and the gradient's hue breath, both in turns.
#[derive(Default)]
pub(crate) struct WashPhase {
    phase: f32,
    /// Where the hue breath is in its cycle, in turns — [`HUE_MULT`] times
    /// slower than `phase` and read through [`Self::hue_deg`].
    hue: f32,
    /// When the last DRIFTING frame was stamped. Cleared whenever the wash
    /// holds, so the first frame after a hold contributes nothing and the
    /// still time in between is never paid back.
    last_ms: Option<u64>,
}

impl WashPhase {
    /// This frame's phase. Advances by the time since the previous drawn
    /// frame at `pace` ms per revolution ([`pace`] decides which), holds on
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
        self.phase = (self.phase + dt as f32 / pace as f32).fract();
        let hue_pace = pace.saturating_mul(HUE_MULT);
        self.hue = (self.hue + dt as f32 / hue_pace as f32).fract();
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

    /// This frame's wander, in turns: how far the wash's pools have drifted
    /// off their rigid orbit — leaning together, reaching in and out — and
    /// how far the page has wound into its whirlpool.
    ///
    /// It is the hue clock itself, read raw rather than as a sine, because
    /// the shader takes its own harmonics of it. Reusing the clock rather than
    /// adding a third keeps one set of fences and one supply of frames, and
    /// it is `0.0` at rest, so a page that has never drifted draws the plain
    /// orbit.
    pub(crate) fn wander(&self) -> f32 {
        self.hue
    }
}

impl crate::app::CrewApp {
    /// Whether the page's wash should drift on its own this frame.
    ///
    /// Four fences, all of which must pass. The setting, because ambient
    /// motion is a taste and some people want a still window. Motion not off,
    /// which is a genuine off. A theme that actually has a wash to move —
    /// moving a phase nothing reads would buy frames for no pixels. And the
    /// OS focus, because the whole cost of this feature is repainting a window
    /// that would otherwise be asleep, and it is only worth paying while
    /// someone is looking at it.
    pub(crate) fn ambient_drift(&self) -> bool {
        self.config.ambient_drift
            && self.win_focus.unwrap_or(true)
            && crate::motion::level() != MotionLevel::Off
            && crew_theme::theme().modern.is_some_and(|m| m.wash > 0.0)
    }
}

#[cfg(test)]
#[path = "washphase_tests.rs"]
mod tests;
