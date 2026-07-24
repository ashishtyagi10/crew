//! Auto-update scheduler: decides WHEN the silent background check runs.
//! Pure Instant arithmetic — the network/disk work stays on the update
//! worker thread (`updatefetch`), so the winit thread never blocks.
use std::time::{Duration, Instant};

/// Launch settles first; the first quiet check runs shortly after startup.
pub(crate) const FIRST_CHECK: Duration = Duration::from_secs(30);
/// Steady-state cadence between quiet checks.
pub(crate) const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// The next-check deadline. `take_due` answers "fire now?" and re-arms.
pub(crate) struct AutoUpdate {
    next_check: Instant,
}

impl AutoUpdate {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            next_check: now + FIRST_CHECK,
        }
    }
    /// True once per elapsed deadline; re-arms to the 6 h cadence.
    pub(crate) fn take_due(&mut self, now: Instant) -> bool {
        if now < self.next_check {
            return false;
        }
        self.next_check = now + CHECK_EVERY;
        true
    }
}

impl Default for AutoUpdate {
    /// `CrewApp` derives `Default`; this mirrors that by arming from "now" —
    /// production startup effectively does the same via `AutoUpdate::new`.
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_check_waits_the_launch_delay_then_rearms_six_hourly() {
        let t0 = Instant::now();
        let mut a = AutoUpdate::new(t0);
        assert!(!a.take_due(t0), "not due immediately at launch");
        assert!(!a.take_due(t0 + FIRST_CHECK - Duration::from_secs(1)));
        assert!(a.take_due(t0 + FIRST_CHECK), "due after the launch delay");
        assert!(
            !a.take_due(t0 + FIRST_CHECK),
            "take_due re-arms — not due twice"
        );
        assert!(!a.take_due(t0 + FIRST_CHECK + CHECK_EVERY - Duration::from_secs(1)));
        assert!(a.take_due(t0 + FIRST_CHECK + CHECK_EVERY));
    }
}
