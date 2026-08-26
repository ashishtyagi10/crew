//! Does the OS want more contrast? — one switch, several answers.
//!
//! macOS has an accessibility switch (Settings → Accessibility → Display →
//! "Increase contrast") that is a request, not a theme: it does not say "use
//! these colours", it says *whatever you are drawing, make it easier to tell
//! apart*. Apps that honor it darken text, strengthen borders, and stop
//! relying on faint tints to carry meaning. Crew ignoring it was awkward
//! precisely because crew is otherwise careful about contrast — it measures
//! every derived role against a WCAG floor (see [`crate::readable`]) and then
//! held those floors fixed no matter what the user had asked the system for.
//!
//! So the switch moves the floors. Everything else falls out of that, because
//! every role crew derives is derived *through* them: the cursor, links,
//! selection, the warning amber, the sparkline. Two things that are not
//! derived move too, and both for the same reason — they are effects that
//! spend contrast:
//!
//! * the **spotlight** over unfocused panes (dimming text is the opposite of
//!   the request), and
//! * the **gradient wash** on the page, which lifts the background the ink
//!   sits on and has only 4–16% headroom over it to begin with.
//!
//! Held in an atomic and read at the point of use, like every other live
//! theme fact, because the floors are consulted while building frames.
use std::sync::atomic::{AtomicBool, Ordering};

static HIGH: AtomicBool = AtomicBool::new(false);

/// Publish the OS "increase contrast" answer (or the user's override).
pub fn set_high_contrast(on: bool) {
    HIGH.store(on, Ordering::Relaxed);
}

/// Whether crew is currently drawing for high contrast.
pub fn high_contrast() -> bool {
    HIGH.load(Ordering::Relaxed)
}

/// WCAG AA for text, 4.5 — or **AAA, 7.0**, when the OS asks for more.
///
/// AAA is the right target for the raised floor rather than some number in
/// between: it is the standard's own next step, it is what "increase
/// contrast" means in the only vocabulary that has one, and every role here
/// is small text or a cursor, which is exactly what AAA was written for.
pub fn text_floor() -> f32 {
    if high_contrast() {
        7.0
    } else {
        4.5
    }
}

/// WCAG AA for non-text marks, 3.0 — or 4.5 when the OS asks for more, which
/// is the text floor: a mark you have to see becomes a mark you have to see
/// *easily*.
pub fn mark_floor() -> f32 {
    if high_contrast() {
        4.5
    } else {
        3.0
    }
}

/// How far an effect that spends contrast — the spotlight wash, the page
/// gradient — is allowed to go, as a multiplier on its own strength.
///
/// Not zero. Killing the spotlight outright would take away the one cue that
/// says which pane is focused, which is itself an accessibility loss; a third
/// of the wash still reads as a hierarchy while costing a fraction of the
/// contrast.
pub fn effect_scale() -> f32 {
    if high_contrast() {
        0.33
    } else {
        1.0
    }
}

/// Serialize the tests that move the global.
///
/// The flag changes what every derived role in [`crate::readable`] comes out
/// as, so a test that flips it races every test that reads one. One lock, held
/// for the whole of any case that touches it — the same discipline the theme
/// global already needs.
#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The switch has to actually move things, and in the direction the user
    /// asked. A floor pair that did not rise, or an effect that did not back
    /// off, is the feature shipped as a no-op.
    #[test]
    fn asking_for_contrast_raises_the_floors_and_quiets_the_effects() {
        let _g = test_lock();
        set_high_contrast(false);
        let (t0, m0, e0) = (text_floor(), mark_floor(), effect_scale());
        set_high_contrast(true);
        let (t1, m1, e1) = (text_floor(), mark_floor(), effect_scale());
        set_high_contrast(false);

        assert!(t1 > t0, "text floor {t0} -> {t1}");
        assert!(m1 > m0, "mark floor {m0} -> {m1}");
        assert!(e1 < e0, "effects {e0} -> {e1}");
        // The ordinary floors must stay exactly the WCAG AA bands crew's
        // whole derivation contract is written against.
        assert_eq!((t0, m0, e0), (4.5, 3.0, 1.0));
        // And the raised ones must be the standard's next band, not a number
        // someone liked the look of.
        assert_eq!((t1, m1), (7.0, 4.5));
    }

    /// A mark never has to clear more than text does — that ordering is what
    /// makes the two bands mean anything.
    #[test]
    fn a_mark_is_never_asked_for_more_than_text() {
        let _g = test_lock();
        for on in [false, true] {
            set_high_contrast(on);
            assert!(mark_floor() <= text_floor(), "at high={on}");
        }
        set_high_contrast(false);
    }

    /// The spotlight must not be switched off: it is the cue that says which
    /// pane has focus, and losing it is itself an accessibility regression.
    #[test]
    fn the_effects_are_quieted_not_killed() {
        let _g = test_lock();
        set_high_contrast(true);
        assert!(effect_scale() > 0.0);
        set_high_contrast(false);
    }
}
