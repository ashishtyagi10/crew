//! Hydrate missing provider env vars from the user's login shell. The crew app
//! is often launched from a GUI or a long-lived terminal whose environment
//! predates the current shell config (e.g. `DASHSCOPE_API_KEY` added to
//! `~/.zshenv` after that environment was created) — discovery would then
//! silently fall back to the wrong provider. At broker startup we ask
//! `$SHELL -ilc env` (bounded; killed on timeout) for the *current* shell env
//! and import only the provider vars missing here; existing process vars
//! always win, so explicit `CREW_PROVIDER=… crew` overrides still hold.
//! `CREW_SHELL_ENV=0` disables the probe (the e2e harness sets it so tests
//! never inherit a developer's real keys).
use std::time::Duration;

/// Provider-relevant vars worth importing: every key discovery looks for,
/// plus every `CREW_*` knob (provider pin, model chains, endpoints, budgets).
///
/// Read from `credentials::VARS` rather than listed here. Providers are a
/// table now (`discover::DIRECT`), and a hand-kept copy of that table in a
/// `matches!` would go stale the first time a row was added — silently, since
/// the only symptom is a key in the user's shell config that crew never picks
/// up, which looks exactly like no key at all.
fn interesting(key: &str) -> bool {
    crate::credentials::VARS.contains(&key) || key.starts_with("CREW_")
}

/// Parse `env` output, keeping `KEY=VALUE` lines that are interesting,
/// non-empty, and `missing` from the current process environment.
fn merge(output: &str, missing: impl Fn(&str) -> bool) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(k, v)| !v.is_empty() && interesting(k) && missing(k))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Which stored credentials should be imported, given a reader of the current
/// process environment. Split out from [`hydrate`] so the precedence rule is
/// testable without mutating process-global state.
///
/// A variable already exported non-empty into this process WINS — that is the
/// most deliberate signal a user can send. Everything else the store holds is
/// imported, which puts it AHEAD of the login-shell pass below: a key typed
/// into crew beats a stale value in a shell rc file, or the user would type a
/// key, see nothing change, and have no way to find out why.
fn credential_imports(
    store: &crate::credentials::Store,
    current: impl Fn(&str) -> Option<String>,
) -> Vec<(String, String)> {
    store
        .keys
        .iter()
        .filter(|(k, v)| {
            !v.is_empty()
                && crate::credentials::VARS.contains(&k.as_str())
                && current(k).is_none_or(|cur| cur.is_empty())
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Variables [`hydrate`] copied out of the credential store into this
/// process's environment.
///
/// They are exported rather than merely resolved, because CHILD processes
/// inherit this environment — a plugin agent that shells out to a CLI reads
/// `ANTHROPIC_API_KEY` from it, and would otherwise never see a key the user
/// typed into crew. But crew's OWN lookups must not treat them as user intent:
/// `hydrate` runs once per broker process, so a key rotated in a later session
/// would be shadowed forever by the value crew itself injected at startup, and
/// the user would face unfixable 401s until they quit. For these variables the
/// store stays the source of truth (see `discover::key_for`).
static CREW_INJECTED: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// Whether `var`'s value in this process's environment was put there by
/// [`hydrate`] from the credential store, rather than by the user.
pub(crate) fn crew_injected(var: &str) -> bool {
    CREW_INJECTED
        .get()
        .is_some_and(|vars| vars.iter().any(|k| k == var))
}

/// Import missing provider vars from the login shell into this process. Must
/// run before the broker spawns any thread (`set_var` is process-global). A
/// hung or odd shell is harmless: the probe is killed after the timeout and
/// discovery proceeds on the inherited env, exactly as before.
pub(crate) fn hydrate() {
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        return;
    }
    // Deliberately after the CREW_SHELL_ENV=0 gate: that switch exists so the
    // e2e harness never inherits a developer's real keys, and stored
    // credentials are exactly as much "the developer's real keys" as their
    // shell env is.
    let imported = credential_imports(&crate::credentials::load(), |k| std::env::var(k).ok());
    for (k, v) in &imported {
        std::env::set_var(k, v);
    }
    let _ = CREW_INJECTED.set(imported.into_iter().map(|(k, _)| k).collect());
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string());
    // Interactive + login so both ~/.zshenv/~/.zprofile and ~/.zshrc exports
    // are visible (keys commonly live in either).
    let args: Vec<String> = ["-i", "-l", "-c", "env"].map(String::from).into();
    let Ok(out) = super::run::run_cli(&shell, &args, Duration::from_secs(3)) else {
        return;
    };
    let missing = |k: &str| std::env::var(k).map_or(true, |v| v.is_empty());
    for (k, v) in merge(&out, missing) {
        std::env::set_var(k, v);
    }
}

#[cfg(test)]
#[path = "shellenv_tests.rs"]
mod tests;
