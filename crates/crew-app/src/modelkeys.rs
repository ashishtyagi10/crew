//! Which provider the broker will pick, discovered the same way it does. The
//! broker hydrates missing provider keys from the login shell before deciding
//! (`crew-plugin`'s `broker/shellenv.rs`), so reading this process's env alone
//! would under-report on a Finder-launched app. Mirrors
//! [`crate::cmdcheck::init_shell_path`]: one `$SHELL -ilc env` on a background
//! thread (NEVER the winit thread), cached in a `OnceLock`.
//! `CREW_SHELL_ENV=0` skips the probe, matching the broker's switch.
//!
//! [`init_probe`] is wired from `main.rs`, beside `init_shell_path`, now that
//! the `/model` picker (`crate::modelpick`) consumes [`provider_now`]. The
//! subprocess is bounded the same way `crew_plugin::broker::shellenv::hydrate`
//! bounds its probe: stdout is drained on a side thread while the spawning
//! thread polls for a result, and the child is killed if a blocking rc file
//! hasn't produced output within [`PROBE_TIMEOUT`] — `Command::output()` has
//! no such deadline and would otherwise park this thread for the process
//! lifetime.
use std::collections::HashSet;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// How long the login-shell probe gets before it's killed and the process env
/// is used as-is. Matches the broker's `shellenv::hydrate` bound.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

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

/// Merge shell environment into probed state, applying the broker's
/// precedence rules: existing process vars always win. Only adopts a shell
/// value when the process didn't supply one. See [`crate::broker::shellenv`].
fn merge_shell_env(probed: &mut Probed, shell_output: &str) {
    for (k, v) in shell_output.lines().filter_map(|l| l.split_once('=')) {
        if !v.is_empty() && KEYS.contains(&k) {
            probed.keys.insert(k.to_string());
            // Process vars always win: only adopt the shell value if absent.
            if k == "CREW_PROVIDER" && probed.provider_pin.is_none() {
                probed.provider_pin = Some(v.to_string());
            }
        }
    }
}

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
        if let Some(text) = bounded_shell_env(&shell, PROBE_TIMEOUT) {
            merge_shell_env(&mut probed, &text);
        }
        let _ = SHELL_PROBE.set(probed);
    });
}

/// Run `shell -ilc env`, killing it if it hasn't produced output within
/// `timeout`. A blocking rc file (a stray prompt, a hung network mount) must
/// never park this thread forever — `Command::output()` alone waits on pipe
/// EOF with no deadline, so stdout is drained on a side thread while this one
/// polls for either a result or the deadline.
fn bounded_shell_env(shell: &str, timeout: Duration) -> Option<String> {
    let mut child = Command::new(shell)
        .args(["-ilc", "env"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut s = String::new();
        let mut r = stdout;
        let _ = r.read_to_string(&mut s);
        let _ = tx.send(s);
    });
    let deadline = Instant::now() + timeout;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(out) => {
                let _ = child.wait();
                return Some(out);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.wait();
                return None;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
    }
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
/// `Route::Unknown` — no row is dimmed on a guess. Consumed by the `/model`
/// picker (`crate::modelpick`).
pub(crate) fn provider_now() -> (Option<crew_plugin::Provider>, bool) {
    let Some(probed) = SHELL_PROBE.get() else {
        return (None, false);
    };
    (resolve(&probed.keys, probed.provider_pin.as_deref()), true)
}

#[cfg(test)]
#[path = "modelkeys_tests.rs"]
mod tests;
