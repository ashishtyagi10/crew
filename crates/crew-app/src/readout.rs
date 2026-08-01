//! Numbers that count rather than jump.
//!
//! A cost or a token total that snaps from one value to the next reads as a
//! repaint; the same number sweeping to its new value reads as an instrument.
//! This module holds the small amount of state that difference needs — the
//! value currently *shown*, as distinct from the value that is *true*.
//!
//! **Counters belong to the surface that draws them.** A first attempt kept
//! them in a process-wide registry keyed by name, which read fine until you
//! noticed that two chat panes would then share one `footer.cost` and fight
//! over it — and which the footer's own tests caught immediately by leaking
//! values between cases. They live on the pane instead. `Cell` gives the
//! interior mutability that needs, since the whole footer is rendered from a
//! `&ChatPane`.
//!
//! **Every counter settles.** A target that stops moving is reached and the
//! timeline goes quiet, which is what lets `poll` stop asking for frames. A
//! counter chasing a value that changes every frame (a token total during a
//! stream) stays live while that is true and settles within one duration of it
//! stopping — bounded, never perpetual.
use std::cell::Cell;

use crate::ease::Timeline;

/// How long a readout takes to reach a new value.
const COUNT_MS: u64 = 420;

/// Differences below this are not worth animating — snapping avoids a
/// perpetual crawl toward a value that keeps drifting by a rounding error.
const EPSILON: f64 = 1e-6;

/// One animated number.
#[derive(Debug, Default)]
pub(crate) struct Counter {
    from: Cell<f64>,
    to: Cell<f64>,
    timeline: Cell<Timeline>,
    seen: Cell<bool>,
}

impl Counter {
    /// The value to display right now, given that the true value is `target`.
    ///
    /// A changed target sweeps from wherever the display had got to, so a value
    /// that moves twice in quick succession never jumps backwards. The **first**
    /// sight of a value is settled rather than a count-up from zero: sweeping
    /// every number up at launch made the footer's rendering depend on how many
    /// times it had been rendered before, which is not a property a footer
    /// should have. Only a value that *changes* sweeps.
    pub(crate) fn tick(&self, target: f64, now: u64) -> f64 {
        if !self.seen.get() {
            self.seen.set(true);
            self.from.set(target);
            self.to.set(target);
            self.timeline.set(Timeline::default());
            return target;
        }
        if (self.to.get() - target).abs() > EPSILON {
            self.from.set(self.value(now));
            self.to.set(target);
            self.timeline
                .set(Timeline::start(now, COUNT_MS, crate::motion::level()));
        }
        self.value(now)
    }

    fn value(&self, now: u64) -> f64 {
        let t = f64::from(self.timeline.get().eased(now, crate::ease::out_cubic));
        self.from.get() + (self.to.get() - self.from.get()) * t
    }

    /// Whether this counter still has frames to draw.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.timeline.get().live(now)
    }
}

/// The animated numbers on one chat pane's summary footer.
#[derive(Debug, Default)]
pub(crate) struct Readouts {
    pub(crate) cost: Counter,
    pub(crate) tok_in: Counter,
    pub(crate) tok_out: Counter,
    pub(crate) bar_5h: Counter,
    pub(crate) ctx: Counter,
}

impl Readouts {
    pub(crate) fn any_live(&self, now: u64) -> bool {
        [
            &self.cost,
            &self.tok_in,
            &self.tok_out,
            &self.bar_5h,
            &self.ctx,
        ]
        .iter()
        .any(|c| c.live(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::MotionLevel;

    fn counter() -> Counter {
        crate::motion::set_level(MotionLevel::Full);
        Counter::default()
    }

    /// A number seen for the first time is simply shown.
    #[test]
    fn first_sight_is_settled() {
        let _g = crate::app::motion_test_guard();
        let c = counter();
        assert_eq!(c.tick(100.0, 1_000), 100.0);
        assert!(!c.live(1_000));
    }

    #[test]
    fn a_changed_value_sweeps_to_it() {
        let _g = crate::app::motion_test_guard();
        let c = counter();
        let now = 1_000;
        assert_eq!(c.tick(100.0, now), 100.0);
        assert_eq!(c.tick(200.0, now), 100.0, "sweep starts where it was");
        let mid = c.tick(200.0, now + COUNT_MS / 2);
        assert!(mid > 100.0 && mid < 200.0, "mid was {mid}");
        assert_eq!(c.tick(200.0, now + COUNT_MS), 200.0);
    }

    /// The point of the module: a counter that arrives stops asking for frames.
    #[test]
    fn a_reached_value_settles() {
        let _g = crate::app::motion_test_guard();
        let c = counter();
        c.tick(42.0, 0);
        c.tick(99.0, 0);
        assert!(c.live(1));
        c.tick(99.0, COUNT_MS);
        assert!(!c.live(COUNT_MS), "an arrived counter must go quiet");
    }

    /// A second change mid-sweep continues from where the display had got to;
    /// restarting from the old *target* would visibly jump backwards.
    #[test]
    fn a_change_mid_sweep_continues_from_the_shown_value() {
        let _g = crate::app::motion_test_guard();
        let c = counter();
        c.tick(0.0, 0);
        c.tick(100.0, 0);
        let shown = c.tick(100.0, COUNT_MS / 2);
        let after = c.tick(200.0, COUNT_MS / 2);
        assert!(
            (after - shown).abs() < 1e-9,
            "display jumped from {shown} to {after}"
        );
    }

    /// An unchanged target must not restart the sweep — a value read every
    /// frame would otherwise animate forever and the app would never idle.
    #[test]
    fn an_unchanged_target_does_not_restart() {
        let _g = crate::app::motion_test_guard();
        let c = counter();
        c.tick(7.0, 0);
        for f in 1..40 {
            c.tick(7.0, f * 20);
        }
        assert!(!c.live(COUNT_MS + 1), "a steady value kept the app awake");
    }

    #[test]
    fn motion_off_shows_the_true_value_immediately() {
        let _g = crate::app::motion_test_guard();
        let c = counter();
        crate::motion::set_level(MotionLevel::Off);
        c.tick(55.0, 900);
        assert_eq!(c.tick(70.0, 900), 70.0, "no sweep at off");
        assert!(!c.live(900));
        crate::motion::set_level(MotionLevel::Full);
    }

    /// Two panes animate their own numbers. The global registry this replaced
    /// would have had them share one counter and overwrite each other.
    #[test]
    fn counters_are_per_surface() {
        let _g = crate::app::motion_test_guard();
        let (a, b) = (Readouts::default(), Readouts::default());
        a.cost.tick(10.0, 0);
        a.cost.tick(20.0, 0);
        b.cost.tick(500.0, 0);
        assert_eq!(b.cost.tick(500.0, 0), 500.0, "b must not see a's sweep");
        assert!(a.any_live(1) && !b.any_live(1));
    }
}
