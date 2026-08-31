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
#[path = "readout_tests.rs"]
mod tests;
