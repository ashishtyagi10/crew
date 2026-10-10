//! When the gradient's hue breath may move, and at what pace — the fence
//! around [`crate::washphase`]'s clock.

/// How much slower than the theme's `drift_ms` the clock turns while a pane
/// works: twice, a revolution every twelve seconds.
pub(crate) const BUSY_MULT: u64 = 2;

/// Ms per revolution this frame, or `None` to hold where it is: the busy
/// pace while a pane works, held otherwise — and always held on a theme
/// with no drift period.
pub(crate) fn pace(drift_ms: u64, busy: bool) -> Option<u64> {
    (busy && drift_ms > 0).then(|| drift_ms.saturating_mul(BUSY_MULT))
}
