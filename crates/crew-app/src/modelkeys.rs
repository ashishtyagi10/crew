//! Which provider the broker will pick, discovered the same way it does. The
//! broker hydrates missing provider keys from the login shell before deciding
//! (`crew-plugin`'s `broker/shellenv.rs`), so reading this process's env alone
//! would under-report on a Finder-launched app. Mirrors
//! [`crate::cmdcheck::init_shell_path`]: one bounded `$SHELL -ilc env` on a
//! background thread (NEVER the winit thread), cached in a `OnceLock`.
//! `CREW_SHELL_ENV=0` skips the probe, matching the broker's switch.
use std::collections::HashSet;
use std::sync::OnceLock;

/// Provider vars worth probing — the same set `shellenv::interesting` imports.
const KEYS: &[&str] = &[
    "DASHSCOPE_API_KEY",
    "OPENROUTER_API_KEY",
    "ANTHROPIC_API_KEY",
    "CREW_PROVIDER",
    "CREW_BROKER_MOCK_REPLY",
];

/// Names seen non-empty in the login shell, once the probe lands.
static SHELL_KEYS: OnceLock<HashSet<String>> = OnceLock::new();

/// Kick off the probe. Call once at startup, beside `init_shell_path`.
pub(crate) fn init_probe() {
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        // No probe: fall back to this process's env immediately.
        let _ = SHELL_KEYS.set(process_keys());
        return;
    }
    std::thread::spawn(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut found = process_keys();
        if let Ok(out) = std::process::Command::new(&shell)
            .args(["-ilc", "env"])
            .output()
        {
            if let Ok(text) = String::from_utf8(out.stdout) {
                for (k, v) in text.lines().filter_map(|l| l.split_once('=')) {
                    if !v.is_empty() && KEYS.contains(&k) {
                        found.insert(k.to_string());
                    }
                }
            }
        }
        let _ = SHELL_KEYS.set(found);
    });
}

/// Keys already non-empty in this process.
fn process_keys() -> HashSet<String> {
    KEYS.iter()
        .filter(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()))
        .map(|k| (*k).to_string())
        .collect()
}

/// The provider the broker would pick, and whether the probe has landed.
/// Before it lands the answer is `(None, false)` and every row reads
/// `Route::Unknown` — no row is dimmed on a guess. Consumed by the /model
/// picker (a later task in this series); dead by clippy's count until then.
#[allow(dead_code)]
pub(crate) fn provider_now() -> (Option<crew_plugin::Provider>, bool) {
    let Some(keys) = SHELL_KEYS.get() else {
        return (None, false);
    };
    let force = std::env::var("CREW_PROVIDER")
        .ok()
        .filter(|v| !v.is_empty());
    (
        crew_plugin::active_provider(force.as_deref(), |k| keys.contains(k)),
        true,
    )
}
