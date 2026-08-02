//! The one resolution function: which provider serves smith work right now,
//! decided rung by rung over the registry — explicit pin, then signed-in
//! subscriptions (CLI-delegated), then API keys, then installed CLIs, then
//! nothing. The mock (test harness) short-circuits everything, exactly as
//! `pick_provider` has always let it. Every signal is injected, so the
//! precedence is table-testable without a process environment, a credential
//! store, or a real CLI on PATH.

use super::registry::{self, CliSpec};

/// The rung resolution landed on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Resolved {
    /// The deterministic test provider.
    Mock,
    /// A signed-in (or pinned) CLI-delegated provider: smith work routes
    /// through `agent` via the existing CLI-relay machinery.
    Delegated {
        name: &'static str,
        agent: &'static str,
    },
    /// An API-key provider, by registry name — served by the existing
    /// `discover::pick_provider` machinery, byte-for-byte.
    Keyed(&'static str),
    /// No provider, but a delegated CLI is installed (signed-out or state
    /// unknown): the keyless relay rung, today's fallback.
    Relay,
    /// Nothing can serve.
    None,
}

/// The injected signals resolution reads, one per rung.
pub(crate) struct Signals<'a> {
    /// The mock provider is active (`CREW_BROKER_MOCK_REPLY`).
    pub mock: bool,
    /// The `CREW_PROVIDER` pin (env or stored), if any.
    pub pin: Option<String>,
    /// Whether a key for this env var resolves (env or credential store).
    pub has_key: &'a dyn Fn(&str) -> bool,
    /// Whether this delegated CLI reports itself signed in (consent-based
    /// probe of the CLI's own status command — never its token store).
    pub signed_in: &'a dyn Fn(&CliSpec) -> bool,
    /// Whether this CLI binary is installed at all.
    pub cli_installed: &'a dyn Fn(&str) -> bool,
}

/// Resolve the discovery order over the registry. Byte-compatible with
/// today's behavior when no subscription exists: mock first, then the pin,
/// then keys in the historic order, then the keyless CLI relay.
pub(crate) fn resolve(s: &Signals) -> Resolved {
    if s.mock {
        return Resolved::Mock;
    }
    // The pin: a delegated name routes delegated (signed in or not — the pin
    // is the user's explicit word); a keyed name forces that provider exactly
    // as `pick_provider(force, …)` always has; an unknown name falls through
    // to auto-discovery, also exactly as before.
    if let Some(pin) = s.pin.as_deref().filter(|p| !p.is_empty()) {
        if let Some(e) = registry::by_name(pin) {
            if e.delegated() {
                if let Some(cli) = e.cli {
                    return Resolved::Delegated {
                        name: e.name,
                        agent: cli.bin,
                    };
                }
            }
            if e.keyed() {
                return Resolved::Keyed(e.name);
            }
        }
    }
    // Signed-in subscriptions: a CLI-delegated provider whose own CLI
    // reports a live login beats any pasted key — the account the user
    // already pays for should serve before pay-per-token billing does.
    for e in registry::delegated() {
        if let Some(cli) = e.cli {
            if (s.signed_in)(&cli) {
                return Resolved::Delegated {
                    name: e.name,
                    agent: cli.bin,
                };
            }
        }
    }
    // API keys, in the historic discovery order.
    if let Some(e) = registry::keyed()
        .into_iter()
        .find(|e| e.key_var.is_some_and(|v| (s.has_key)(v)))
    {
        return Resolved::Keyed(e.name);
    }
    // Installed (but not signed-in) delegated CLIs: the keyless relay rung.
    if registry::delegated()
        .into_iter()
        .any(|e| e.cli.is_some_and(|c| (s.cli_installed)(c.bin)))
    {
        return Resolved::Relay;
    }
    Resolved::None
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;
