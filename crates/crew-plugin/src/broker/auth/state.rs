//! Live per-provider auth state — the one snapshot behind `/model`'s
//! grouped picker and `/doctor`'s per-provider auth lines, so the two
//! surfaces can never disagree. States only, never values: nothing here
//! ever touches (or could print) a key.

use super::probe::{self, CliAuth};
use super::registry;
use super::Resolved;

/// What one provider's auth looks like right now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AuthState {
    /// Delegated CLI reports a live login — a usable subscription.
    SignedIn,
    /// Delegated CLI installed but signed out (the sign-in affordance).
    SignedOut,
    KeyPresent,
    NoKey,
    /// A CLI on PATH whose signed-in state crew cannot read (an own-auth
    /// CLI like opencode, an old CLI version, or the rung switched off) —
    /// still usable as a relay agent.
    Installed,
    /// Delegated CLI not installed at all.
    Absent,
}

/// One provider's snapshot row.
#[derive(Clone, Debug)]
pub(crate) struct ProviderInfo {
    pub name: String,
    pub state: AuthState,
    /// The exact sign-in command, for delegated providers.
    pub login: Option<&'static str>,
    /// Whether live resolution picked this provider.
    pub active: bool,
}

/// Every registry provider's live state, in registry order, plus any
/// installed CLI the adapter table knows that the registry doesn't
/// (opencode carries its own sign-in and is not a model provider).
pub(crate) fn snapshot() -> Vec<ProviderInfo> {
    let store = crate::credentials::load();
    let active = super::resolved_live();
    let is_active = |n: &str| match &active {
        Resolved::Delegated { name, .. } => *name == n,
        Resolved::Keyed(name) => *name == n,
        _ => false,
    };
    let mut out = Vec::new();
    for e in registry::entries() {
        // The mock is the harness's provider, not a user-facing row.
        if e.name == "mock" {
            continue;
        }
        let state = match e.cli {
            Some(cli) => match probe::state_cached(&cli) {
                CliAuth::SignedIn => AuthState::SignedIn,
                CliAuth::SignedOut => AuthState::SignedOut,
                CliAuth::Absent => AuthState::Absent,
                CliAuth::Unknown if super::super::run::on_path(cli.bin) => AuthState::Installed,
                CliAuth::Unknown => AuthState::Absent,
            },
            None => {
                let keyed = e
                    .key_var
                    .is_some_and(|v| super::super::discover::key_for(&store, v).is_some());
                if keyed {
                    AuthState::KeyPresent
                } else {
                    AuthState::NoKey
                }
            }
        };
        out.push(ProviderInfo {
            name: e.name.to_string(),
            state,
            login: e.cli.map(|c| c.login),
            active: is_active(e.name),
        });
    }
    for a in super::super::agents::known_adapters() {
        if registry::by_name(a.name()).is_none() && a.probe() {
            out.push(ProviderInfo {
                name: a.name().to_string(),
                state: AuthState::Installed,
                login: None,
                active: false,
            });
        }
    }
    out
}
