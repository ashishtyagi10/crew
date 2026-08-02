//! The grant lifecycle after a sign-in (condition 5): a stored access token
//! serves until its expiry, refreshes transparently through the registry's
//! token endpoint, and on a HARD failure discards itself and arms exactly
//! one re-auth prompt — then silence until the user signs in again.
//!
//! `key_stand_in` is the whole wiring surface: `discover::key_for` falls
//! through to it, so a fresh grant IS a key as far as classify, the planner,
//! workers, judges and the roster are concerned — none of them changed.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crew_hive::deviceflow::{self, DeviceEndpoints};

use super::device::{endpoints_for, runtime};
use super::registry;
use super::tokens::{self, StoredToken};

/// A valid access token (+ the resource host the grant named) for a
/// device-flow provider — refreshing when expired. `None` means no usable
/// grant: never signed in, or a hard refresh failure (which discards the
/// dead grant and arms the one re-auth prompt, so this stays quiet — no
/// repeat HTTP — until the user acts).
pub(crate) fn fresh_access(provider: &str) -> Option<(String, Option<String>)> {
    let tok = tokens::load(provider)?;
    let e = endpoints_for(provider)?;
    fresh_with(provider, &e, tok, tokens::now_secs(), &mut |p, t| match t {
        Some(t) => {
            let _ = tokens::store(p, t);
        }
        None => tokens::clear(p),
    })
}

/// The testable half of [`fresh_access`]: verdict over an explicit grant and
/// clock. `store` receives `Some(new grant)` on a successful refresh and
/// `None` when the grant is dead and must be discarded.
pub(crate) fn fresh_with(
    provider: &str,
    e: &DeviceEndpoints,
    tok: StoredToken,
    now: u64,
    store: &mut dyn FnMut(&str, Option<StoredToken>),
) -> Option<(String, Option<String>)> {
    if tokens::is_fresh(&tok, now) {
        return Some((tok.access, tok.resource));
    }
    let (refresh, rt) = match (tok.refresh, runtime()) {
        (Some(r), Ok(rt)) => (r, rt),
        _ => {
            store(provider, None);
            mark_reauth(provider);
            return None;
        }
    };
    match rt.block_on(deviceflow::device_refresh(e, &refresh)) {
        Ok(t) => {
            let mut st = tokens::stored_from(&t, now);
            // Servers may omit these on refresh; the old values stay good.
            st.refresh = st.refresh.or(Some(refresh));
            st.resource = st.resource.or(tok.resource);
            let out = (st.access.clone(), st.resource.clone());
            store(provider, Some(st));
            Some(out)
        }
        Err(_) => {
            // Status deliberately unreported here: the ONE re-auth prompt is
            // the surface, and an error string would invite logging it.
            store(provider, None);
            mark_reauth(provider);
            None
        }
    }
}

/// An access token standing in for the API key in `var`, when the provider
/// declares a device flow and a grant is stored — the OAuth rung of
/// `discover::key_for`. Never logs the token.
pub(crate) fn key_stand_in(var: &str) -> Option<String> {
    let name = registry::name_for_var(var)?;
    registry::by_name(name)?.device?;
    fresh_access(name).map(|(token, _)| token)
}

/// The chat endpoint the stored grant is valid against, when the grant named
/// one — Qwen tokens serve at `portal.qwen.ai`, not at the key-shaped
/// DashScope endpoint. `None` when there is no grant or it named no host
/// (callers keep their configured endpoint).
pub(crate) fn grant_chat_url(provider: &str) -> Option<String> {
    Some(chat_endpoint(&tokens::load(provider)?.resource?))
}

/// Normalize a token response's `resource_url` — a bare host or a base URL —
/// into a full OpenAI-wire chat endpoint.
pub(crate) fn chat_endpoint(resource: &str) -> String {
    let base = if resource.contains("://") {
        resource.trim_end_matches('/').to_string()
    } else {
        format!("https://{}", resource.trim_end_matches('/'))
    };
    if base.ends_with("/chat/completions") {
        base
    } else if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

/// Re-auth prompt state, per provider: a hard refresh failure arms it, the
/// surface TAKES it exactly once (then silence), a fresh sign-in disarms it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reauth {
    Needed,
    Prompted,
}

fn reauth() -> &'static Mutex<HashMap<String, Reauth>> {
    static S: OnceLock<Mutex<HashMap<String, Reauth>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Arm the one re-auth prompt — unless it was already shown (no nag loop).
pub(crate) fn mark_reauth(provider: &str) {
    let mut m = reauth().lock().unwrap_or_else(|e| e.into_inner());
    m.entry(provider.to_string()).or_insert(Reauth::Needed);
}

/// `true` exactly once per armed prompt: the caller prints its line and the
/// state moves to `Prompted` until a new sign-in clears it.
pub(crate) fn take_reauth(provider: &str) -> bool {
    let mut m = reauth().lock().unwrap_or_else(|e| e.into_inner());
    match m.get(provider) {
        Some(Reauth::Needed) => {
            m.insert(provider.to_string(), Reauth::Prompted);
            true
        }
        _ => false,
    }
}

pub(crate) fn clear_reauth(provider: &str) {
    let mut m = reauth().lock().unwrap_or_else(|e| e.into_inner());
    m.remove(provider);
}

/// The one armed re-auth line across every device-flow provider, taken at
/// most once each — what the task worker prints before dispatch.
pub(crate) fn reauth_note() -> Option<String> {
    registry::entries()
        .iter()
        .filter(|e| e.device.is_some())
        .find(|e| take_reauth(e.name))
        .map(|e| {
            format!(
                "{} sign-in expired \u{2014} open /model and pick it to sign in again",
                e.name
            )
        })
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod tests;
