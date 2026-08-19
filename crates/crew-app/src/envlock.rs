//! Test-only serialisation for environment mutation. These variables are
//! process-global state: under the default parallel test runner, any test
//! that sets one races every test that reads it (real-path history saves,
//! `~` expansion). One crate-wide lock, taken by [`with_vars`], serialises
//! them all — a per-module lock only protects a module against itself.
//! Mirrors `palette::test_guard` / `app::theme_test_guard`.
#![cfg(test)]

use std::ffi::OsString;
use std::path::Path;

/// The variables that have to move for `dirs::home_dir()` to follow. `HOME`
/// is enough on Unix; Windows ignores it entirely and reads `USERPROFILE`,
/// so setting only `HOME` there left every "isolated" test writing to the
/// real profile — history tests read back hundreds of entries other tests
/// had pushed, and the suite quietly littered the developer's own home.
#[cfg(unix)]
pub(crate) const HOME_VARS: &[&str] = &["HOME"];
#[cfg(not(unix))]
pub(crate) const HOME_VARS: &[&str] = &["HOME", "USERPROFILE"];

/// Run `f` with each `(name, value)` set, holding the crate-wide lock and
/// restoring every prior value before releasing it.
pub(crate) fn with_vars<T>(vars: &[(&str, OsString)], f: impl FnOnce() -> T) -> T {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev: Vec<_> = vars
        .iter()
        .map(|(k, _)| (*k, std::env::var_os(k)))
        .collect();
    for (k, v) in vars {
        std::env::set_var(k, v);
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

/// Run `f` with the home-directory variables pointed at `home`.
///
/// This redirects `dirs::home_dir()`, and with it anything derived from
/// `$HOME` on Unix — but NOT `dirs::config_dir()` on Windows, which resolves
/// through the Known Folder API and ignores the environment entirely. A
/// store that lives under the config dir therefore needs its own path
/// override to be isolated on Windows; see `farpane::cmdhist::path`.
pub(crate) fn with_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let vars: Vec<_> = HOME_VARS
        .iter()
        .map(|k| (*k, OsString::from(home)))
        .collect();
    with_vars(&vars, f)
}
