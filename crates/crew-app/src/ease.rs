//! The easing curve and bounded timelines — the motion vocabulary.
//!
//! Separate from [`crate::anim`], which owns the shared clock and the two
//! primitives (`tri`, `lerp_rgb`) that predate it. Everything here is a pure
//! function of a timestamp, so motion never needs a timer, a thread or a sleep
//! — all of which would land on the winit thread and stall every pane.
//!
//! Every timeline is **bounded**: it reports [`Timeline::live`] false once it
//! has run its course, which is what lets `poll` stop scheduling redraws. An
//! animation that never finishes would repaint an idle crew forever, and "idle
//! costs nothing" is the invariant the whole render loop is built on.
use crate::motion::MotionLevel;

/// Ease-out cubic: fast departure, soft landing. The default for anything that
/// *arrives* (a bracket drawing in, a card assembling).
pub(crate) fn out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// A bounded animation window: starts at `start_ms`, runs for `dur_ms`.
///
/// Construct with [`Timeline::start`] so the duration is scaled by the user's
/// Motion setting in one place — at `MotionLevel::Off` the duration collapses
/// to zero, so `progress` is immediately 1.0 and `live` is immediately false.
/// Off is then a real off: the final state draws once and nothing reschedules.
/// `Default` is the settled state — both fields zero, so `progress` is 1.0 and
/// `live` is false — which is what an app that has never animated should hold.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Timeline {
    start_ms: u64,
    dur_ms: u64,
}

impl Timeline {
    /// Begin a timeline of `dur_ms` (at full motion) from `now`.
    pub(crate) fn start(now: u64, dur_ms: u64, level: MotionLevel) -> Self {
        Self {
            start_ms: now,
            dur_ms: level.scale_ms(dur_ms),
        }
    }

    /// Linear progress in `0.0..=1.0`. A zero-length timeline is complete.
    pub(crate) fn progress(&self, now: u64) -> f32 {
        if self.dur_ms == 0 {
            return 1.0;
        }
        let elapsed = now.saturating_sub(self.start_ms);
        (elapsed as f32 / self.dur_ms as f32).clamp(0.0, 1.0)
    }

    /// Whether this timeline still has frames to draw. `poll` schedules
    /// redraws off exactly this, so a timeline that never settles is a bug
    /// that shows up as a crew that never sleeps.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.dur_ms > 0 && now.saturating_sub(self.start_ms) < self.dur_ms
    }

    /// Progress through `f` — the usual way to read a timeline.
    pub(crate) fn eased(&self, now: u64, f: fn(f32) -> f32) -> f32 {
        f(self.progress(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_spans_zero_to_one_and_clamps() {
        assert!(out_cubic(0.0).abs() < 1e-6, "must start at 0");
        assert!((out_cubic(1.0) - 1.0).abs() < 1e-5, "must end at 1");
        assert!(out_cubic(-5.0).abs() < 1e-6);
        assert!((out_cubic(5.0) - 1.0).abs() < 1e-5);
    }

    /// Ease-out is ahead of linear in its first half — that is what "fast
    /// departure" means, and it is the whole reason to use it.
    #[test]
    fn out_cubic_leads_linear() {
        assert!(out_cubic(0.25) > 0.25);
        assert!(out_cubic(0.5) > 0.5);
    }

    #[test]
    fn timeline_runs_then_settles() {
        let t = Timeline::start(1_000, 300, MotionLevel::Full);
        assert_eq!(t.progress(1_000), 0.0);
        assert!((t.progress(1_150) - 0.5).abs() < 1e-6);
        assert_eq!(t.progress(1_300), 1.0);
        assert!(t.live(1_299));
        assert!(
            !t.live(1_300),
            "a settled timeline must stop scheduling frames"
        );
    }

    /// The reduce-motion contract: Off doesn't animate faster, it doesn't
    /// animate at all — one frame at the final state, then silence.
    #[test]
    fn motion_off_settles_instantly() {
        let t = Timeline::start(1_000, 300, MotionLevel::Off);
        assert_eq!(t.progress(1_000), 1.0);
        assert!(!t.live(1_000));
    }

    #[test]
    fn subtle_is_quicker_than_full() {
        let now = 500;
        let subtle = Timeline::start(now, 400, MotionLevel::Subtle);
        let full = Timeline::start(now, 400, MotionLevel::Full);
        assert!(subtle.progress(now + 200) > full.progress(now + 200));
    }

    /// Progress is clamped, so a timeline read long after it ended reports its
    /// final state rather than running away.
    /// The `Default` timeline is the resting state, not a pending one.
    #[test]
    fn default_is_already_settled() {
        let t = Timeline::default();
        assert_eq!(t.progress(0), 1.0);
        assert_eq!(t.progress(u64::MAX), 1.0);
        assert!(!t.live(0));
    }

    #[test]
    fn progress_never_exceeds_one() {
        let t = Timeline::start(0, 100, MotionLevel::Full);
        assert_eq!(t.progress(10_000), 1.0);
        assert!((t.eased(10_000, out_cubic) - 1.0).abs() < 1e-6);
    }

    /// Clock reads before the start (possible when a timeline is stamped from
    /// a slightly later `now`) must not underflow into a huge progress value.
    #[test]
    fn progress_before_start_is_zero() {
        let t = Timeline::start(1_000, 100, MotionLevel::Full);
        assert_eq!(t.progress(0), 0.0);
    }
}
