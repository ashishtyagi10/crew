//! Where the live hint mode ([`crate::hints`]) is kept.
//!
//! The app has one mode for the whole process. Under test that one slot was
//! shared by every test thread, and the mode is keyed by pane INDEX: while
//! `hintact_tests` held it open on pane 0, any test drawing its own app's
//! pane 0 at the same moment got the hint labels and the `a copies · A opens`
//! legend stamped over it. `helpscrim_tests` read exactly that — a pane whose
//! top border said `crats` and whose last row was the hint legend — and only
//! on its last theme, the one that ran while `hintact_tests` did. So tests
//! get a slot per thread (each test is its own thread), and a test sees only
//! the mode it opened.
use crate::hints::Hints;
use std::sync::Mutex;

/// The process's one slot.
#[cfg(not(test))]
pub(crate) fn slot() -> &'static Mutex<Option<Hints>> {
    static MODE: Mutex<Option<Hints>> = Mutex::new(None);
    &MODE
}

/// This test thread's slot: leaked once per thread so the guard can be
/// `'static` like the app's, which a test never notices (a few bytes).
#[cfg(test)]
pub(crate) fn slot() -> &'static Mutex<Option<Hints>> {
    thread_local! {
        static MODE: &'static Mutex<Option<Hints>> = Box::leak(Box::default());
    }
    MODE.with(|m| *m)
}

#[cfg(test)]
#[path = "hintslot_tests.rs"]
mod tests;
