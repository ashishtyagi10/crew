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

/// What the probe found: which vars were non-empty, and — since it's the
/// decisive input to routing, not just a presence check — `CREW_PROVIDER`'s
/// actual value. Mirrors [`crate::cmdcheck::init_shell_path`] /
/// `effective_path`: capture the real shell value, don't just note it existed.
struct Probed {
    keys: HashSet<String>,
    provider_pin: Option<String>,
}

/// Probe results, once the probe lands.
static SHELL_PROBE: OnceLock<Probed> = OnceLock::new();

/// Kick off the probe. Call once at startup, beside `init_shell_path`.
pub(crate) fn init_probe() {
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        // No probe: fall back to this process's env immediately.
        let _ = SHELL_PROBE.set(process_probe());
        return;
    }
    std::thread::spawn(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut probed = process_probe();
        if let Ok(out) = std::process::Command::new(&shell)
            .args(["-ilc", "env"])
            .output()
        {
            if let Ok(text) = String::from_utf8(out.stdout) {
                for (k, v) in text.lines().filter_map(|l| l.split_once('=')) {
                    if !v.is_empty() && KEYS.contains(&k) {
                        probed.keys.insert(k.to_string());
                        if k == "CREW_PROVIDER" {
                            probed.provider_pin = Some(v.to_string());
                        }
                    }
                }
            }
        }
        let _ = SHELL_PROBE.set(probed);
    });
}

/// Keys and `CREW_PROVIDER` value already visible in this process, before the
/// probe lands (or when it's skipped).
fn process_probe() -> Probed {
    let keys = KEYS
        .iter()
        .filter(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()))
        .map(|k| (*k).to_string())
        .collect();
    let provider_pin = std::env::var("CREW_PROVIDER")
        .ok()
        .filter(|v| !v.is_empty());
    Probed { keys, provider_pin }
}

/// Pure resolution logic: given the keys visible to the broker and an
/// optional forced provider name, which provider would `active_provider`
/// pick? Extracted from [`provider_now`] so it's unit-testable without a
/// process-global `OnceLock` (which can't be reset between tests).
fn resolve(keys: &HashSet<String>, forced: Option<&str>) -> Option<crew_plugin::Provider> {
    crew_plugin::active_provider(forced, |k| keys.contains(k))
}

/// The provider the broker would pick, and whether the probe has landed.
/// Before it lands the answer is `(None, false)` and every row reads
/// `Route::Unknown` — no row is dimmed on a guess. Consumed by the /model
/// picker (a later task in this series); dead by clippy's count until then.
#[allow(dead_code)]
pub(crate) fn provider_now() -> (Option<crew_plugin::Provider>, bool) {
    let Some(probed) = SHELL_PROBE.get() else {
        return (None, false);
    };
    (resolve(&probed.keys, probed.provider_pin.as_deref()), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_probed_pin_is_honoured() {
        let keys: HashSet<String> = ["OPENROUTER_API_KEY".to_string()].into_iter().collect();
        assert_eq!(
            resolve(&keys, Some("openrouter")),
            Some(crew_plugin::Provider::OpenRouter)
        );
    }

    #[test]
    fn pin_absent_from_process_env_but_present_in_probe_is_still_honoured() {
        // Simulates a GUI-launched process: this process never saw
        // CREW_PROVIDER, but the login-shell probe found it in an rc file.
        let keys: HashSet<String> = ["ANTHROPIC_API_KEY".to_string()].into_iter().collect();
        // Without the pin, auto-discovery lands on the only key present.
        assert_eq!(resolve(&keys, None), Some(crew_plugin::Provider::Anthropic));
        // With the probed pin honoured, the explicit forced provider wins
        // instead — even though no OPENROUTER_API_KEY is in the key set.
        assert_eq!(
            resolve(&keys, Some("openrouter")),
            Some(crew_plugin::Provider::OpenRouter)
        );
    }

    #[test]
    fn key_order_auto_discovery_applies_when_there_is_no_pin() {
        let keys: HashSet<String> = ["DASHSCOPE_API_KEY".to_string()].into_iter().collect();
        assert_eq!(resolve(&keys, None), Some(crew_plugin::Provider::DashScope));
        assert_eq!(resolve(&HashSet::new(), None), None);
    }
}
