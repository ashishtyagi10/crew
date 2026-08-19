//! Test-only serialisation for `$HOME` mutation. The variable is
//! process-global state: under the default parallel test runner, any test
//! that sets it races every test that reads it (real-path history saves,
//! `~` expansion). One crate-wide lock, taken by [`with_home`], serialises
//! them all — a per-module lock only protects a module against itself.
//! Mirrors `palette::test_guard` / `app::theme_test_guard`.
#![cfg(test)]

/// The variables that have to move for `dirs::home_dir()` to follow. `HOME`
/// is enough on Unix; Windows ignores it entirely and reads `USERPROFILE`,
/// so setting only `HOME` there left every "isolated" test writing to the
/// real profile — history tests read back hundreds of entries other tests
/// had pushed, and the suite quietly littered the developer's own home.
#[cfg(unix)]
const HOME_VARS: &[&str] = &["HOME"];
#[cfg(not(unix))]
const HOME_VARS: &[&str] = &["HOME", "USERPROFILE"];

/// Run `f` with the home-directory variables pointed at `home`, holding the
/// crate-wide lock and restoring their prior values before releasing it.
pub(crate) fn with_home<T>(home: &std::path::Path, f: impl FnOnce() -> T) -> T {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev: Vec<_> = HOME_VARS
        .iter()
        .map(|k| (*k, std::env::var_os(k)))
        .collect();
    for k in HOME_VARS {
        std::env::set_var(k, home);
    }
    let out = f();
    for (k, v) in prev {
        match v {
            Some(p) => std::env::set_var(k, p),
            None => std::env::remove_var(k),
        }
    }
    out
}
