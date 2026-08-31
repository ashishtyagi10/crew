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
#[path = "autoupdate_tests.rs"]
mod tests;
