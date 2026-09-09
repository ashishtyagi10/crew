//! The CLI-minted rung: a vendor CLI owns the sign-in (`ant auth login`, a
//! browser OAuth against the Claude Console) and, on request, MINTS a
//! short-lived bearer for whoever asks (`ant auth print-credentials
//! --access-token`, which refreshes first when it must). Crew asks — the
//! CLI's own documented command, consent-based, exactly like the delegated
//! rung's status probe — and presents the bearer through its native
//! provider. Crew never opens `~/.config/anthropic/`: the CLI is the sole
//! authority on the profile, and its own `logout` is the sign-out.
//!
//! A minted token is cached in-process for [`MINT_TTL_SECS`] so `key_for`
//! (called once per model request) does not spawn a process per request.
//! TOKEN VALUES ARE NEVER PRINTED — no `Debug`, no error string, no log line.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::probe::{self, CliAuth};
use super::registry::{self, CliSpec};

/// The minting half of a `CliMinted` entry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MintSpec {
    /// The CLI's status probe + login command (`auth::probe` reads it).
    pub cli: CliSpec,
    /// argv (after the binary) that prints ONE bare access token on stdout.
    pub mint: &'static [&'static str],
    /// The exact sign-out command — the CLI owns the store, so crew's
    /// `/logout` hands the user this instead of deleting anything.
    pub logout: &'static str,
    /// How to get the CLI when it isn't installed.
    pub install: &'static str,
}

/// The Anthropic CLI. `ant auth status` always exits 0 and names the active
/// credential source; a signed-in Console profile prints `user_oauth`.
pub(crate) const ANT: MintSpec = MintSpec {
    cli: CliSpec {
        bin: "ant",
        status: &["auth", "status"],
        login: "ant auth login",
        signed_in_marker: Some("user_oauth"),
    },
    mint: &["auth", "print-credentials", "--access-token"],
    logout: "ant auth logout",
    install: "brew install anthropics/tap/ant",
};

/// Re-mint after this long. `print-credentials` refreshes an expiring token
/// itself, so the window only bounds the process spawns.
pub(crate) const MINT_TTL_SECS: u64 = 300;
/// A mint may refresh over the network before it prints.
const MINT_TIMEOUT: Duration = Duration::from_secs(20);

/// A minted access token: one bare token on stdout from a successful run.
/// A failure, prose (`not logged in …`), several lines, or the JSON form
/// of `print-credentials` is no token — nothing else ever reaches a header.
pub(crate) fn parse_token(ok: bool, stdout: &str) -> Option<String> {
    let t = stdout.trim();
    let tokenish = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    (ok && !t.is_empty() && t.chars().all(tokenish)).then(|| t.to_string())
}

/// A mint runner: `(bin, args)` → `Some((exit-ok, stdout))`, or `None` on
/// spawn failure / timeout — injected so the cache is table-testable.
pub(crate) type MintRunner<'a> = &'a dyn Fn(&str, &[&str]) -> Option<(bool, String)>;

pub(crate) struct Minted {
    token: String,
    at: u64,
}
pub(crate) type Cache = HashMap<&'static str, Minted>;

/// The cached token while it is younger than the TTL, else a fresh mint
/// (cached on success only, so a failed run is retried next time). Pure
/// over its cache and clock.
pub(crate) fn fresh_with(
    cache: &mut Cache,
    name: &'static str,
    spec: &MintSpec,
    now: u64,
    run: MintRunner,
) -> Option<String> {
    if let Some(m) = cache.get(name) {
        if now.saturating_sub(m.at) < MINT_TTL_SECS {
            return Some(m.token.clone());
        }
    }
    let (ok, out) = run(spec.cli.bin, spec.mint)?;
    let token = parse_token(ok, &out)?;
    cache.insert(
        name,
        Minted {
            token: token.clone(),
            at: now,
        },
    );
    Some(token)
}

fn cache() -> &'static Mutex<Cache> {
    static C: OnceLock<Mutex<Cache>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A valid access token for a CLI-minted provider — only while its CLI
/// reports a live sign-in (the cached probe: no spawn at all when signed
/// out). A mint that fails despite the sign-in arms the one re-auth prompt.
pub(crate) fn fresh_access(name: &str) -> Option<String> {
    let e = registry::by_name(name)?;
    let spec = e.mint?;
    if probe::state_cached(&spec.cli) != CliAuth::SignedIn {
        return None;
    }
    let mut c = cache().lock().unwrap_or_else(|e| e.into_inner());
    let got = fresh_with(&mut c, e.name, &spec, super::tokens::now_secs(), &|b, a| {
        probe::run_split(b, a, MINT_TIMEOUT).map(|(ok, out, _stderr)| (ok, out))
    });
    match got {
        Some(_) => super::refresh::clear_reauth(e.name),
        None => super::refresh::mark_reauth(e.name),
    }
    got
}

/// An access token standing in for the API key in `var` — the minted rung
/// of `discover::key_for`. Never logs the token.
pub(crate) fn key_stand_in(var: &str) -> Option<String> {
    let name = registry::name_for_var(var)?;
    fresh_access(name)
}

#[cfg(test)]
#[path = "mint_tests.rs"]
mod tests;
