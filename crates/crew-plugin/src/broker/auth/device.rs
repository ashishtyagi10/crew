//! The native device-flow engine (condition 4 of the 2026-08-01 goal): run
//! an RFC 8628 sign-in against a registry entry's declared endpoints, honor
//! the server's polling contract (`interval`, `slow_down`, expiry), hand the
//! granted tokens to storage, and refresh them transparently until a hard
//! failure marks the ONE re-auth prompt.
//!
//! Blocking by design and ALWAYS run on a worker thread (a background task
//! or the refresh path inside a model call) — never the stdio loop, never
//! the pane. The sleeper and the storage handoff are injected so every
//! timing rule is table-testable as recorded numbers, no real clock.
use std::time::Duration;

use crew_hive::deviceflow::{self, DeviceEndpoints, DevicePoll};

use super::registry::{self, DeviceSpec};
use super::tokens::{self, StoredToken};

/// Hard ceiling on one sign-in wait, whatever the server's `expires_in`
/// says: a quarter hour of polling is patience, more is a leak.
pub(crate) const MAX_WAIT_SECS: u64 = 900;
/// RFC 8628 §3.5: every `slow_down` adds this to the polling interval.
pub(crate) const SLOW_DOWN_BUMP_SECS: u64 = 5;

/// What the pane's code card shows — the deliberately visible fields only.
pub(crate) struct SignInCard {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
}

/// How one sign-in attempt ended. `Failed`'s text is an outermost error
/// message only (status codes, never bodies or codes — `deviceflow`'s rule).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    SignedIn,
    Expired,
    Denied,
    TimedOut,
    /// The user cancelled (`/stop`) between polls.
    Stopped,
    Failed(String),
}

/// The registry entry's endpoints for `name`, if it declares a device flow.
/// `CREW_OAUTH_BASE` rebases both URLs onto a stub server — the e2e seam;
/// tests never call the registry's live URLs.
pub(crate) fn endpoints_for(name: &str) -> Option<DeviceEndpoints> {
    let d = registry::by_name(name)?.device?;
    Some(endpoints_from(&d, std::env::var("CREW_OAUTH_BASE").ok()))
}

fn endpoints_from(d: &DeviceSpec, base: Option<String>) -> DeviceEndpoints {
    let (device_url, token_url) = match base.as_deref().filter(|b| !b.is_empty()) {
        Some(b) => (format!("{b}/device"), format!("{b}/token")),
        None => (d.device_url.to_string(), d.token_url.to_string()),
    };
    DeviceEndpoints {
        device_url,
        token_url,
        client_id: d.client_id.to_string(),
        scope: d.scope.to_string(),
    }
}

/// Total polling budget for a flow: the server's word, capped.
pub(crate) fn poll_budget(expires_in: u64) -> u64 {
    expires_in.min(MAX_WAIT_SECS)
}

pub(crate) fn runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

/// Run one full sign-in: start, show the card, poll to a verdict, hand the
/// grant to `store`. `sleep` is the injected clock (tests record durations;
/// live passes a sliced `std::thread::sleep`); `cancelled` is checked once
/// per iteration so `/stop` ends the wait between polls, never mid-request.
pub(crate) fn run_flow(
    provider: &str,
    e: &DeviceEndpoints,
    emit_card: &mut dyn FnMut(SignInCard),
    sleep: &mut dyn FnMut(Duration),
    store: &mut dyn FnMut(&str, StoredToken),
    cancelled: &dyn Fn() -> bool,
) -> Outcome {
    let rt = match runtime() {
        Ok(rt) => rt,
        Err(err) => return Outcome::Failed(err.to_string()),
    };
    let pkce = crew_hive::pkce();
    let start = match rt.block_on(deviceflow::device_start(e, Some(&pkce.challenge))) {
        Ok(s) => s,
        Err(err) => return Outcome::Failed(err.to_string()),
    };
    emit_card(SignInCard {
        user_code: start.user_code.clone(),
        verification_uri: start.verification_uri.clone(),
        verification_uri_complete: start.verification_uri_complete.clone(),
    });
    let budget = poll_budget(start.expires_in);
    let mut interval = start.interval.max(1);
    let mut elapsed = 0u64;
    loop {
        if cancelled() {
            return Outcome::Stopped;
        }
        if elapsed + interval > budget {
            return Outcome::TimedOut;
        }
        sleep(Duration::from_secs(interval));
        elapsed += interval;
        match rt.block_on(deviceflow::device_poll(
            e,
            &start.device_code,
            Some(&pkce.verifier),
        )) {
            Ok(DevicePoll::Ready(t)) => {
                store(provider, tokens::stored_from(&t, tokens::now_secs()));
                super::refresh::clear_reauth(provider);
                return Outcome::SignedIn;
            }
            Ok(DevicePoll::Pending) => {}
            Ok(DevicePoll::SlowDown) => interval += SLOW_DOWN_BUMP_SECS,
            Ok(DevicePoll::Expired) => return Outcome::Expired,
            Ok(DevicePoll::Denied) => return Outcome::Denied,
            Err(err) => return Outcome::Failed(err.to_string()),
        }
    }
}

#[cfg(test)]
#[path = "device_tests.rs"]
mod tests;
