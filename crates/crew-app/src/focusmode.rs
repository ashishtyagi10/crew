//! Focus mode: crew stops interrupting, and the page says so.
//!
//! Crew is built to get your attention — a pane that goes quiet toasts, a
//! pane blocked on a prompt raises a marker and can pull focus to itself, a
//! bell rings. That is right almost all of the time and exactly wrong for the
//! twenty minutes someone is trying to finish a thought in one pane. The
//! usual answer is `/notify off`, which is a different thing: it *drops* the
//! events, so afterwards you cannot tell what you missed.
//!
//! Focus mode holds them instead. While it is on:
//!
//! * **nothing pops.** Notifications still fire, still write the LOG, still
//!   raise the pane's own attention marker — they simply do not step onto the
//!   canvas as cards. The count is kept.
//! * **nothing steals.** The blocked-pane detector still badges a pane
//!   waiting on a human, but it never moves focus there. Being yanked into
//!   another pane mid-sentence is the single most expensive interruption crew
//!   can produce.
//! * **the rest of the canvas recedes.** The spotlight over unfocused panes
//!   deepens, so the pane you chose is unmistakably the one you are in.
//!
//! Leaving says what happened: one card, one line, `3 held while focused` —
//! so the mode costs you awareness only until you come out of it. The whole
//! point is that nothing was thrown away.
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether focus mode is on, as a global.
///
/// Read from the render path (the spotlight is applied per pane per frame),
/// so it rides an atomic for the same reason `density::gap` and
/// `poleshift::poles` do rather than being threaded down the scene chain.
static ON: AtomicBool = AtomicBool::new(false);

/// How far unfocused content leans toward the page while focused, against
/// [`crate::spotlight`]'s ordinary 15%.
///
/// Chosen against the same constraint the ambient wash was: unfocused panes
/// must stay READABLE. This is emphasis, not an overlay — a glance at a
/// neighbouring agent's output has to still work, or the mode costs the
/// peripheral awareness that made a grid worth having.
pub(crate) const DIM: f32 = 0.42;

pub(crate) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

pub(crate) fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// The unfocused-content wash to use right now.
pub(crate) fn dim() -> f32 {
    // Dimming text is the opposite of what "increase contrast" asks for, so
    // the wash backs off when the OS asks — quieted, never killed: the
    // spotlight is the cue that says which pane has focus, and losing that is
    // itself an accessibility loss.
    let base = if on() { DIM } else { crate::spotlight::DIM };
    base * crew_theme::contrast::effect_scale()
}

/// What was held while focus mode was on.
#[derive(Default)]
pub(crate) struct Held {
    /// Toasts suppressed since the mode was entered.
    pub(crate) toasts: usize,
}

impl Held {
    /// The line to show on leaving, or `None` when nothing was held — a
    /// summary of zero is a notification about not being notified.
    pub(crate) fn summary(&self) -> Option<String> {
        match self.toasts {
            0 => None,
            1 => Some("1 notification held while focused".to_string()),
            n => Some(format!("{n} notifications held while focused")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mode is only worth having if it actually deepens the wash, and
    /// only usable if it stops short of hiding the rest of the grid.
    #[test]
    fn focus_deepens_the_spotlight_without_erasing_it() {
        let _g = crate::app::motion_test_guard();
        set(false);
        assert_eq!(dim(), crate::spotlight::DIM);
        set(true);
        assert_eq!(dim(), DIM);
        // Compile-time: these are the contract on the constant itself, not on
        // any run of the code.
        const _: () = assert!(DIM > crate::spotlight::DIM);
        const _: () = assert!(DIM < 0.6);
        set(false);
    }

    /// Zero held is not news. Anything else has to be counted exactly — the
    /// whole promise of holding over dropping is that the number is true.
    #[test]
    fn the_summary_counts_and_stays_quiet_at_zero() {
        assert_eq!(Held::default().summary(), None);
        assert_eq!(
            Held { toasts: 1 }.summary().as_deref(),
            Some("1 notification held while focused")
        );
        assert_eq!(
            Held { toasts: 4 }.summary().as_deref(),
            Some("4 notifications held while focused")
        );
    }
}
