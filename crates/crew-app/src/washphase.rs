//! The clock behind the modern backdrop's gradient wash: where the two pools
//! of pole light sit on their orbit this frame.
//!
//! The wash itself is drawn by the background pass (see crew-render's
//! `ModernPaper`); all that lives here are the clocks it moves by — the
//! orbit (which also breathes the pools and turns the lattice's tint), the
//! slower wander/hue clock that sways the silk, the eddies' own loop and how
//! awake the flow is — all driven by one flywheel. The rule for when they may
//! move, their **pace** in ms per revolution or `None` to hold, lives in
//! [`crate::washgate`].
//!
//! The phase is advanced from ELAPSED TIME BETWEEN DRAWN FRAMES rather than
//! read off the wall clock, so it never jumps: a wall-clock phase would
//! teleport the pools across the page after a quiet minute, while accumulating
//! deltas means the motion is continuous across every pause.
//!
//! ## Two paces
//!
//! **Busy** rides frames that activity was already drawing, so it costs
//! nothing: a revolution per [`crate::washgate::BUSY_MULT`] times the theme's
//! `drift_ms` (twelve seconds). Stepping between the two is a glide, not a
//! jump: the clocks' speed follows the pace through a flywheel
//! ([`SPIN_UP_MS`]).
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

/// How much slower the page's EDDIES loop than the pools orbit: φ², an
/// irrational ratio, so the eddies never fall into step with the other two
/// clocks and the page as a whole never repeats — at the themes' 6 s
/// `drift_ms`, a loop every ~63 s while the room is quiet. The other two run
/// two to one and alone would replay the same 48 seconds forever.
const EDDY_MULT: f32 = 2.618_034;

/// How quickly the clocks' SPEED follows a change of pace, in ms: the time
/// constant of the flywheel. A pane that starts working sways the silk up
/// to its busy pace over a couple of seconds rather than lurching to twice
/// the speed in one frame, and when the work ends it coasts back down
/// over several — slower down than up, the way a heavy wheel spins up under
/// power and then coasts.
const SPIN_UP_MS: f32 = 900.0;
const SPIN_DOWN_MS: f32 = 2_800.0;

/// How long the page takes to wake into its full flow, in ms of drift: the
/// current and eddies bending in and the silk's folds brightening from
/// nothing, rather than a still page snapping into motion on its first
/// drifted frame.
const WAKE_MS: f32 = 3_000.0;

/// How much slower the idle drift is than the busy one. At the themes'
/// 6 s `drift_ms` this is a revolution every 24 seconds — 15° of orbit per
/// second, a pool breath every 12 — which reads as a room whose light is
/// plainly alive without anything asking for your attention. Busy motion is
/// still the faster signal. (It was 15, a
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

#[cfg(test)]
use crate::washgate::pace;

/// The wash's orbital position and the gradient's hue breath, both in turns.
#[derive(Default)]
pub(crate) struct WashPhase {
    phase: f32,
    /// Where the hue breath is in its cycle, in turns — [`HUE_MULT`] times
    /// slower than `phase` and read through [`Self::hue_deg`].
    hue: f32,
    /// How far into its wake-up the page is, `0.0..=1.0` — linear in drifted
    /// time, eased in [`Self::live`]. It only ever rises: once awake, the
    /// flow never stops on a clock, it just holds when the wash holds.
    wake: f32,
    /// Where the eddies are in their loop, in turns — [`EDDY_MULT`] times
    /// slower than `phase`.
    eddy: f32,
    /// How fast the orbit is turning, in turns per ms — eased toward the
    /// pace's rate (see [`SPIN_UP_MS`]). `0.0` until the first drift, which
    /// starts at its pace outright: the wake already eases a still page in.
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
        // Every clock rides the one flywheel, so they speed up and coast
        // together and keep their ratios.
        let step = self.rate * dt;
        self.phase = (self.phase + step).fract();
        self.hue = (self.hue + step / HUE_MULT as f32).fract();
        self.eddy = (self.eddy + step / EDDY_MULT).fract();
        self.wake = (self.wake + dt / WAKE_MS).min(1.0);
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
    /// how far the silk has swayed.
    ///
    /// It is the hue clock itself, read raw rather than as a sine, because
    /// the shader takes its own harmonics of it. Reusing the clock rather than
    /// adding a third keeps one set of fences and one supply of frames, and
    /// it is `0.0` at rest, so a page that has never drifted draws the plain
    /// orbit.
    pub(crate) fn wander(&self) -> f32 {
        self.hue
    }

    /// How awake the page's flow is: `0.0` for a process that has never
    /// drifted (the still page every resting shot draws), easing up to `1.0`
    /// over the first [`WAKE_MS`] of drift and staying there. Eased
    /// (smoothstep) so the motion swells in rather than starting at a lurch.
    pub(crate) fn live(&self) -> f32 {
        self.wake * self.wake * (3.0 - 2.0 * self.wake)
    }

    /// Every clock the backdrop reads this frame, as the renderer takes them.
    pub(crate) fn clocks(&self) -> crew_render::WashClocks {
        crew_render::WashClocks {
            phase: self.phase,
            wander: self.wander(),
            live: self.live(),
            eddy: self.eddy,
        }
    }
}

#[cfg(test)]
#[path = "washphase_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "washlive_tests.rs"]
mod live_tests;
