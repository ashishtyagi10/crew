//! One lock for the tests that read the process-wide theme across a step and
//! the ones that switch it (`color_tests`): a switch between a test's two
//! reads compares a colour floored for one page with another page.
use std::sync::{Mutex, MutexGuard};

static THEME: Mutex<()> = Mutex::new(());

pub(crate) fn lock() -> MutexGuard<'static, ()> {
    THEME.lock().unwrap_or_else(|e| e.into_inner())
}
