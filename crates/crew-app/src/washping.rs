//! The vortex answers you: every line you send (Enter, in any pane) drops a
//! ring of light in from the page's rim to the vortex's eye, and the eye
//! flashes as it lands. This is the stamp the backdrop keys off — process-wide,
//! like the motion level, so the key path need not thread anything through.
use std::sync::atomic::{AtomicU64, Ordering};

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

/// How long a ping is drawn, in ms: the ring's fall to the eye (1.3 s in the
/// shader) and the eye's flash after it.
pub(crate) const LIFE_MS: u64 = 2_400;

/// When Enter was last pressed, on `anim::now_ms`'s clock; 0 for never.
static AT: AtomicU64 = AtomicU64::new(0);

/// Stamp a press of Enter. A held key's repeats are not new lines.
pub(crate) fn key(k: &KeyEvent) {
    if !k.repeat && k.logical_key == Key::Named(NamedKey::Enter) {
        AT.store(crate::anim::now_ms().max(1), Ordering::Relaxed);
    }
}

/// Seconds since the last ping while it is still being drawn, else `-1.0`.
pub(crate) fn age_s(now_ms: u64) -> f32 {
    age_at(AT.load(Ordering::Relaxed), now_ms)
}

/// [`age_s`] for a ping stamped `at` (0 = none), as a pure function.
fn age_at(at: u64, now_ms: u64) -> f32 {
    match (at, now_ms.checked_sub(at)) {
        (0, _) | (_, None) => -1.0,
        (_, Some(d)) if d > LIFE_MS => -1.0,
        (_, Some(d)) => d as f32 / 1000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ping_lives_for_its_fall_and_flash_then_is_gone() {
        assert_eq!(age_at(0, 5_000), -1.0, "never pinged");
        assert_eq!(age_at(10_000, 10_000), 0.0);
        assert_eq!(age_at(10_000, 11_300), 1.3);
        assert_eq!(age_at(10_000, 10_000 + LIFE_MS), LIFE_MS as f32 / 1000.0);
        assert_eq!(age_at(10_000, 10_001 + LIFE_MS), -1.0, "drawn out");
        assert_eq!(age_at(10_000, 9_000), -1.0, "a clock that went back");
    }
}
