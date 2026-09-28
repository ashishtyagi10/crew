//! Opening the provider's connection before the call that needs it.
//!
//! Measured (2026-09-27, DashScope international): TCP and TLS to the host
//! cost ~0.5 s of a turn's first call. The pool keeps one connection per host
//! warm ([`super::io`]), but drops it after 30 s idle ([`super::http_client`])
//! — and 30 s is less than it takes to read a reply and type the next
//! message, so the first call of most turns paid the handshake again. Codex
//! and Claude Code both hide it by connecting early; so does crew, when the
//! user starts typing.
//!
//! The warm is one HEAD to the endpoint on the SAME pooled client the calls
//! use, run on the provider runtime, so the connection it leaves idle is the
//! one the next call checks out. It carries no credentials: opening a socket
//! needs none, and a request that holds no key cannot leak one. Its status is
//! never read — a 404 or a 405 means the handshake is done just as a 200
//! would.
use std::time::Duration;

/// A bound on the warm itself. It is detached, so a host that never answers
/// would otherwise hold a task for the whole read timeout (120 s by default)
/// to produce nothing; a handshake that takes longer than this is not one the
/// next call would have been glad to reuse.
const WARM_TIMEOUT: Duration = Duration::from_secs(15);

/// Open a pooled connection to `url`'s host on `client`, and return at once.
/// Failures are dropped: a warm that could not connect costs the next call
/// nothing it wasn't going to pay anyway, and the call reports the error.
pub(super) fn warm(client: &reqwest::Client, url: &str) {
    let client = client.clone();
    let url = url.to_string();
    super::io::detach(async move {
        if let Ok(resp) = client.head(&url).timeout(WARM_TIMEOUT).send().await {
            // Read to the end: an HTTP/1 connection goes back to the pool
            // only once its response is finished.
            let _ = resp.bytes().await;
        }
    });
}

#[cfg(test)]
#[path = "warm_tests.rs"]
mod tests;
