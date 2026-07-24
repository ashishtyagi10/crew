//! Test-only serialisation for `$HOME` mutation. The variable is
//! process-global state: under the default parallel test runner, any test
//! that sets it races every test that reads it (real-path history saves,
//! `~` expansion). One crate-wide lock, taken by [`with_home`], serialises
//! them all — a per-module lock only protects a module against itself.
//! Mirrors `palette::test_guard` / `app::theme_test_guard`.
#![cfg(test)]

/// Run `f` with `$HOME` set to `home`, holding the crate-wide `$HOME` lock
/// and restoring the prior value before releasing it.
pub(crate) fn with_home<T>(home: &std::path::Path, f: impl FnOnce() -> T) -> T {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var_os("HOME");
    std::env::set_var("HOME", home);
    let out = f();
    match prev {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }
    out
}
