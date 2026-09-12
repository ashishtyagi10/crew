//! The appearance guard: the one lock every test that touches crew's
//! process-global look — the theme, the gradient poles, the motion level —
//! takes before it writes one.
//!
//! Split out of [`crate::app`] for the line cap; `app` re-exports it, so
//! every call site still reads `crate::app::theme_test_guard()`.
#![cfg(test)]

/// Serialises tests that mutate crew-theme's process-global state (`CURRENT`,
/// the random-rotation atomics): several files across this crate exercise
/// `/theme` behaviour (chattheme.rs, toggles.rs, spawn.rs, config.rs) and
/// would otherwise race under the default parallel test runner. Mirrors the
/// `guard()` used by crew-theme's own tests.
pub(crate) static THEME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Holds the lock, pins a known theme, and RESTORES whatever was active on drop.
///
/// The restore is the load-bearing half; the pin only decides what the first taker in a process
/// sees. No test here distinguishes them — a test that tried was found to pass with the pin
/// removed, so it was deleted rather than kept as decoration.
///
/// The lock alone was not enough. Serialising theme-touching tests stops them racing, but it
/// leaves whichever theme the last one published in place for everybody after — and a test that
/// compares against a derived colour (`chatink` floors every ink for contrast against the card)
/// then passes or fails depending on what ran before it. Three markdown colour tests went from
/// green to red overnight with no code change that way, and reproduced under
/// `--test-threads=1`, which is what ruled a race out and pointed here.
pub(crate) struct ThemeGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev: crew_theme::ThemeId,
    /// The gradient poles in force when the guard was taken.
    ///
    /// A guard that put the theme back and left the POLES where a test had
    /// moved them restored half a theme: the canvas keeps its colour from the
    /// pole pair, so a test that only reads `theme()` still sees the light
    /// somebody else turned on. `themepeek`'s previews move both.
    poles: Option<crew_theme::poleshift::Poles>,
    /// The motion level in force when the guard was taken — put back on drop
    /// so each guarded test starts from the same place.
    motion: crate::motion::MotionLevel,
}

thread_local! {
    /// Whether THIS thread holds the appearance guard.
    ///
    /// The lock alone could only serialise the tests that remembered to take
    /// it; it could not tell a test that never mentions motion — a canvas
    /// config test, say — that `share_config` reaches `apply_config` and
    /// flips the global motion level under somebody else's feet. So the knob
    /// ASSERTS against this flag ([`crate::motion::set_level`]), and a
    /// forgotten guard fails deterministically in the test that forgot it
    /// instead of flaking a distant animation test two runs in three.
    static HOLDS_APPEARANCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the calling thread is inside a [`theme_test_guard`].
pub(crate) fn holds_appearance_guard() -> bool {
    HOLDS_APPEARANCE.with(std::cell::Cell::get)
}

impl Drop for ThemeGuard {
    fn drop(&mut self) {
        crew_theme::set_theme(self.prev);
        crew_theme::poleshift::set_custom(self.poles);
        // The motion level goes back too, for the same reason the theme
        // does: a guarded test that never mentions motion must not inherit
        // the level the last one happened to leave. Restored BEFORE the flag
        // drops, because `set_level` asserts on it.
        crate::motion::set_level(self.motion);
        HOLDS_APPEARANCE.with(|h| h.set(false));
    }
}

pub(crate) fn theme_test_guard() -> ThemeGuard {
    let lock = THEME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    HOLDS_APPEARANCE.with(|h| h.set(true));
    let prev = crew_theme::current_id();
    let poles = crew_theme::poleshift::custom();
    let motion = crate::motion::level();
    // The default. A test wanting another theme still sets one after taking the guard; this
    // only decides what a test that never mentions a theme gets, which used to be "whatever the
    // previous test left behind". The poles get the same treatment: the theme's own.
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    crew_theme::poleshift::set_custom(None);
    ThemeGuard {
        _lock: lock,
        prev,
        poles,
        motion,
    }
}

/// Serialises tests that touch the process-global motion level
/// (`motion::LEVEL`): the animation tests across this crate (app_tests,
/// the chatmsgs caret, ghost.rs, readout.rs, paneview_tests, motion.rs)
/// each pin a level and then read it back through timelines; unguarded they
/// race under the parallel runner — an `Off` window opened by one test makes
/// another's just-started timeline instant (`every_animation_terminates` was
/// the observed flake).
///
/// SHARES [`theme_test_guard`]'s lock rather than adding a second one:
/// `apply_config` mutates both globals in one call, so the apply_config
/// tests (holding only the theme guard) flip the motion level too — a
/// separate lock would let a guarded animation test race exactly that. One
/// consequence: a test must take one guard or the other, NEVER both (the
/// mutex is not reentrant).
///
/// Taking it is no longer a thing to REMEMBER: [`crate::motion::set_level`]
/// asserts on [`holds_appearance_guard`], so a test that writes the knob
/// without the guard fails in itself. That mattered because the writes that
/// actually raced came from tests with no interest in motion at all — a
/// canvas `share_config` test and a font-resize wheel test, each reaching
/// `apply_config`, which publishes theme AND motion in one call. Under the
/// old convention the only symptom was `motion_off_schedules_nothing…`
/// failing two runs in three, a test neither of them names.
pub(crate) fn motion_test_guard() -> ThemeGuard {
    theme_test_guard()
}
