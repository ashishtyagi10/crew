//! One login-shell probe, shared by PATH detection and provider-key
//! discovery. A Dock/Finder-launched app inherits launchd's minimal PATH and
//! never sees vars an interactive rc file exports, so both `cmdcheck`'s
//! command detection and the broker's provider pick would under-report
//! reading this process's env alone (`crew-plugin`'s `broker/shellenv.rs`
//! hydrates the same way). A single `$SHELL -ilc env` on a background
//! thread (NEVER the winit thread) covers both in the common case: its
//! output already contains `PATH=` alongside the provider vars, so a second
//! shell just for PATH would be redundant. Was two probes: this module
//! (`modelkeys`, provider keys only) plus `cmdcheck`'s own unbounded
//! `$SHELL -lc` for PATH; now one normally, cached in a `OnceLock`. PATH
//! also switched from `-lc` to `-ilc` (a strict superset, picking up a PATH
//! exported only from `~/.zshrc`) and is now bounded by [`PROBE_TIMEOUT`]
//! rather than unbounded — but `-ilc` sourcing `~/.zshrc` where `-lc` only
//! read `~/.zprofile` measured ~24x slower on one machine, so the variable
//! most likely to blow that budget is PATH, the one value that previously
//! could never time out at all. So on a timeout or spawn error, a second,
//! fast `$SHELL -lc 'printf %s "$PATH"'` runs for PATH alone, itself
//! bounded by [`PATH_FALLBACK_TIMEOUT`]; provider keys stay unrecovered in
//! that case, since [`provider_now`] already degrades to `Unknown` rather
//! than claiming a key that was never actually seen. `CREW_SHELL_ENV=0`
//! skips both shells, matching the broker's switch.
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Duration;

#[path = "shellprobe_shell.rs"]
mod shell;
use shell::{bounded_shell_env, bounded_shell_path};

/// How long the probe gets before it's killed and the process env is used
/// as-is. Matches the broker's `shellenv::hydrate` bound.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// How long the PATH-only fallback gets. Only reached after the primary
/// probe already burned [`PROBE_TIMEOUT`], so this stays short: the fast
/// form measured 0.031s on a normal machine; 1s is generous headroom.
const PATH_FALLBACK_TIMEOUT: Duration = Duration::from_secs(1);

/// Provider vars worth probing, matching `shellenv::interesting`.
///
/// Built from `credentials::VARS` so it cannot drift from the provider table
/// the way a hand-kept list does. The failure mode of drift is invisible: a
/// key sitting in the user's shell config that crew never imports is
/// indistinguishable, from the pane, from having no key at all.
fn keys() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = crew_plugin::credentials::VARS.to_vec();
    v.push("CREW_PROVIDER");
    v.push("CREW_BROKER_MOCK_REPLY");
    v
}

/// What the probe found. No `Debug`: `openrouter_key` is a secret and must
/// never end up in a log line.
///
/// `path` uses the opposite precedence from `provider_pin`/`openrouter_key`
/// (which keep "process always wins", see [`merge_shell_env`]): PATH is the
/// value this process's launchd-minimal PATH is *supposed* to be overridden
/// by, so a non-blank shell PATH replaces it outright; [`process_probe`]
/// leaves this field unset rather than pre-seeding it.
struct Probed {
    keys: HashSet<String>,
    provider_pin: Option<String>,
    openrouter_key: Option<String>,
    path: Option<String>,
}

/// Probe results, once the probe lands.
static SHELL_PROBE: OnceLock<Probed> = OnceLock::new();

/// NAMES of keys supplied from inside crew this session, plus whatever the
/// credential store already held. Holds only variable names, never values:
/// the only reader ([`merge_entered`]) only ever asks "was a non-empty value
/// supplied for this name", so retaining the value itself would just be the
/// whole credential store sitting in process memory for the process
/// lifetime with no reader that benefits. The shell probe's cache is a
/// `OnceLock` and can't be re-set, so rather than convert it, entered names
/// are unioned over it.
static ENTERED: std::sync::LazyLock<std::sync::RwLock<HashSet<String>>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(HashSet::new()));

/// The provider pin the credential store holds — seeded at startup and
/// re-recorded whenever a key is saved this session ([`note_pin`]). The broker
/// has always honoured this pin (`discover::forced_provider`); the app did
/// not, so after saving an OpenRouter key the app kept resolving DashScope
/// from its own fixed discovery order: the Claude row stayed dim asking for
/// the same key, accepting it re-opened the same prompt — forever, across
/// restarts — while the broker was meanwhile routing to OpenRouter. Not a
/// secret (a provider name), but it lives beside [`ENTERED`] for the same
/// reason: [`SHELL_PROBE`] is a `OnceLock` that cannot be re-set.
static PINNED: std::sync::LazyLock<std::sync::RwLock<Option<String>>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(None));

/// Record a key supplied in-app so [`provider_now`] resolves against it
/// immediately. Only the NAME is retained — see [`ENTERED`] — and only when
/// `value` is non-empty; an empty value clears any previously entered name,
/// matching `credentials::save_key`'s "empty value removes the key" rule.
/// NEVER logs the value.
pub(crate) fn note_key(var: &str, value: &str) {
    if let Ok(mut e) = ENTERED.write() {
        if value.is_empty() {
            e.remove(var);
        } else {
            e.insert(var.to_string());
        }
    }
}

/// Record the provider pin `credentials::save_key` just wrote, so
/// [`provider_now`] resolves against it immediately — the same "without a
/// restart" contract [`note_key`] gives the key itself. Blank is not a pin.
pub(crate) fn note_pin(provider: &str) {
    if provider.is_empty() {
        return;
    }
    if let Ok(mut p) = PINNED.write() {
        *p = Some(provider.to_string());
    }
}

/// Union entered key names into a probed key set (the testable half).
fn merge_entered(keys: &mut HashSet<String>, entered: &HashSet<String>) {
    keys.extend(entered.iter().cloned());
}

/// Which pin wins: `CREW_PROVIDER` — from this process or the login-shell
/// probe, both already merged into `Probed::provider_pin` — over the pin the
/// credential store recorded. The pure seam, and deliberately the same rule as
/// `crew_plugin`'s `discover::resolve_forced`: if these two ever disagreed the
/// app would dim rows the broker can serve, or promise ones it cannot.
fn resolve_pin<'a>(env: Option<&'a str>, stored: Option<&'a str>) -> Option<&'a str> {
    env.filter(|v| !v.is_empty())
        .or(stored.filter(|v| !v.is_empty()))
}

/// Merge shell environment into probed state. Provider vars: existing
/// process vars always win, only adopting a shell value when absent (see
/// `crew_plugin`'s `broker/shellenv.rs`). PATH is the opposite — see
/// [`Probed`]. Splits on the *first* `=` per line, since a PATH value can
/// itself contain `=`; multi-line values aren't disambiguated. Prior
/// behaviour for the provider vars, which always came from this `env`
/// dump — but new for PATH, which previously came from a dedicated
/// `printf` unaffected by any other variable's content. Last `PATH=` line
/// wins, which is the right order: rc-file output prints *before* the
/// trailing `env` dump, so anything an rc file echoes gets overwritten by
/// the real, final PATH.
fn merge_shell_env(probed: &mut Probed, shell_output: &str) {
    for (k, v) in shell_output.lines().filter_map(|l| l.split_once('=')) {
        if k == "PATH" {
            if !v.trim().is_empty() {
                probed.path = Some(v.to_string());
            }
            continue;
        }
        if !v.is_empty() && keys().contains(&k) {
            probed.keys.insert(k.to_string());
            // Process vars always win: only adopt the shell value if absent.
            if k == "CREW_PROVIDER" && probed.provider_pin.is_none() {
                probed.provider_pin = Some(v.to_string());
            }
            if k == "OPENROUTER_API_KEY" && probed.openrouter_key.is_none() {
                probed.openrouter_key = Some(v.to_string());
            }
        }
    }
}

/// Kick off the probe. Call exactly once at startup.
///
/// Also seeds [`ENTERED`] and [`PINNED`] from the on-disk credential store, so
/// a key typed into the popup in an earlier session makes its model row live
/// immediately this session too — and routes to the provider it pinned —
/// without waiting on the shell probe.
pub(crate) fn init_probe() {
    let store = crew_plugin::credentials::load();
    for (var, value) in store.keys {
        note_key(&var, &value);
    }
    if let Some(p) = store.provider {
        note_pin(&p);
    }
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        // No probe: fall back to this process's env immediately.
        let _ = SHELL_PROBE.set(process_probe());
        return;
    }
    std::thread::spawn(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut probed = process_probe();
        match bounded_shell_env(&shell, PROBE_TIMEOUT) {
            Some(text) => merge_shell_env(&mut probed, &text),
            // Timed out or failed to spawn: fall back to the fast
            // PATH-only form (see `adopt_fallback_path`).
            None => {
                let fallback = bounded_shell_path(&shell, PATH_FALLBACK_TIMEOUT);
                adopt_fallback_path(&mut probed, fallback.as_deref());
            }
        }
        let _ = SHELL_PROBE.set(probed);
    });
}

/// Adopt `fallback` (the fast probe's raw `$PATH` value, not an `env`-style
/// line) as `probed.path` if non-blank. Pure and spawn-free, extracted so
/// the fallback's trigger — "primary probe produced nothing, so use the
/// fast PATH-only value instead" — is unit-testable without a real shell,
/// like [`merge_shell_env`] and [`resolve`]. The decision to call this at
/// all is the caller matching on [`bounded_shell_env`]'s `Option`.
fn adopt_fallback_path(probed: &mut Probed, fallback: Option<&str>) {
    if let Some(path) = fallback {
        if !path.trim().is_empty() {
            probed.path = Some(path.to_string());
        }
    }
}

/// Keys/pin/key already visible in this process. `path` stays unset — see
/// [`Probed`] for why PATH gets no process-wins fallback.
fn process_probe() -> Probed {
    let keys = keys()
        .iter()
        .filter(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()))
        .map(|k| (*k).to_string())
        .collect();
    let provider_pin = env_nonempty("CREW_PROVIDER");
    let openrouter_key = env_nonempty("OPENROUTER_API_KEY");
    Probed {
        keys,
        provider_pin,
        openrouter_key,
        path: None,
    }
}

/// This process's own value for `k`, or `None` if unset/empty.
fn env_nonempty(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.is_empty())
}

/// Pure resolution logic, extracted from [`provider_now`] so it's
/// unit-testable without the process-global `OnceLock`.
fn resolve(keys: &HashSet<String>, forced: Option<&str>) -> Option<crew_plugin::Provider> {
    crew_plugin::active_provider(forced, |k| keys.contains(k))
}

/// The provider the broker would pick, and whether the probe has landed
/// (`(None, false)` beforehand — no row is dimmed on a guess).
pub(crate) fn provider_now() -> (Option<crew_plugin::Provider>, bool) {
    let Some(probed) = SHELL_PROBE.get() else {
        return (None, false);
    };
    let mut keys = probed.keys.clone();
    if let Ok(entered) = ENTERED.read() {
        merge_entered(&mut keys, &entered);
    }
    let stored = PINNED.read().ok().and_then(|p| p.clone());
    let pin = resolve_pin(probed.provider_pin.as_deref(), stored.as_deref());
    (resolve(&keys, pin), true)
}

/// The OpenRouter key: this process's own env wins if set (same precedence
/// as [`merge_shell_env`]), else the probed value, else `None`. Never log
/// this value: it's a secret.
pub(crate) fn openrouter_key() -> Option<String> {
    match SHELL_PROBE.get() {
        Some(probed) => probed.openrouter_key.clone(),
        None => env_nonempty("OPENROUTER_API_KEY"),
    }
}

/// The PATH detection resolves against: the login-shell PATH once landed
/// and non-blank, else the process PATH.
pub(crate) fn effective_path() -> String {
    let process_path = || std::env::var("PATH").unwrap_or_default();
    match SHELL_PROBE.get() {
        Some(probed) => probed.path.clone().unwrap_or_else(process_path),
        None => process_path(),
    }
}

#[cfg(test)]
#[path = "shellprobe_tests.rs"]
mod tests;
