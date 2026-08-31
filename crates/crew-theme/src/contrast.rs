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
#[path = "contrast_tests.rs"]
mod tests;
