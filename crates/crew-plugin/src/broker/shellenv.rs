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

/// Provider-relevant vars worth importing: the API keys discovery looks for,
/// plus every `CREW_*` knob (provider pin, model chains, endpoints, budgets).
fn interesting(key: &str) -> bool {
    matches!(
        key,
        "DASHSCOPE_API_KEY" | "OPENROUTER_API_KEY" | "ANTHROPIC_API_KEY"
    ) || key.starts_with("CREW_")
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

/// Import missing provider vars from the login shell into this process. Must
/// run before the broker spawns any thread (`set_var` is process-global). A
/// hung or odd shell is harmless: the probe is killed after the timeout and
/// discovery proceeds on the inherited env, exactly as before.
pub(crate) fn hydrate() {
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        return;
    }
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
