//! Provider auth: the registry (which providers exist and how each one
//! authenticates — data, not forks) and the one resolution function that
//! orders them: pin, then signed-in subscriptions, then API keys, then
//! installed CLIs. Consent-based only: the CLI-delegated rung drives a
//! vendor CLI's own commands and never reads another app's token store.

pub(crate) mod device;
pub(crate) mod probe;
pub(crate) mod registry;
pub(crate) mod resolve;
pub(crate) mod state;
pub(crate) mod tokens;

pub(crate) use resolve::{resolve, Resolved, Signals};

/// [`resolve`] over the live machine: the credential store + process
/// environment for the pin and keys (read once per call, like every other
/// discovery path), the cached CLI probes for subscriptions, PATH for
/// installs. The probe closures run lazily — under the mock or a pin the
/// resolution returns before any process is spawned.
pub(crate) fn resolved_live() -> Resolved {
    let store = crate::credentials::load();
    resolve(&Signals {
        mock: super::discover::key_for(&store, "CREW_BROKER_MOCK_REPLY").is_some(),
        pin: super::discover::forced_provider(&store),
        has_key: &|v| super::discover::key_for(&store, v).is_some(),
        signed_in: &|c| probe::state_cached(c) == probe::CliAuth::SignedIn,
        cli_installed: &|b| super::run::on_path(b),
    })
}

/// Who leads a plain (unaddressed) task, subscription rung included: a
/// signed-in (or pinned) CLI-delegated provider's own CLI takes the lead
/// through the EXISTING relay machinery — exactly what `@claude <task>`
/// does, minus the typing — and everything else falls through to
/// `stdio::keyless_starter`, today's behavior byte-for-byte.
pub(crate) fn starter(session: &super::session::Session) -> Option<String> {
    starter_for(&resolved_live(), session.registry().names())
        .or_else(|| super::stdio::keyless_starter(session))
}

/// The pure half of [`starter`]: the roster name the delegated resolution
/// picks, if it picks one.
fn starter_for(r: &Resolved, names: Vec<String>) -> Option<String> {
    match r {
        Resolved::Delegated { agent, .. } => {
            names.into_iter().find(|n| n.eq_ignore_ascii_case(agent))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod live_tests;
