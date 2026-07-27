//! OpenRouter browser sign-in: the loopback callback parser.
//!
//! This module holds the pure, I/O-free logic for interpreting the single
//! HTTP request the loopback listener receives once the user finishes
//! authorizing in their browser. Keeping it free of sockets means the
//! trickiest part of the flow — matching the nonce, picking a `code` or
//! `error` out of the query string — is fully unit-testable before any
//! listener exists.

/// What the single callback request turned out to be.
///
/// `#[cfg(test)]`: this binary crate has no external API surface, so a
/// `pub(crate)` item with no caller anywhere in the tree is provably dead —
/// `cargo clippy --all-targets -- -D warnings` fails on it otherwise. Task 3
/// appends the listener (`await_callback`), which calls `parse_request` from
/// real (non-test) code; remove this `#[cfg(test)]` (and the one below) the
/// moment that caller lands.
#[cfg(test)]
#[derive(Debug, PartialEq)]
pub(crate) enum Callback {
    Code(String),
    Denied(String),
    /// Not ours, or carries nothing usable — answer 404 and keep waiting.
    Ignore,
}

/// Parse the request line of the one callback we expect.
///
/// `nonce` guards the port: it is loopback, but any process on this machine
/// can connect to it, so a request whose path is not exactly our nonce is not
/// ours. This is the `state` equivalent for a localhost callback.
///
/// `#[cfg(test)]`: see the note on `Callback` above — drop this once Task 3's
/// `await_callback` calls it for real.
#[cfg(test)]
pub(crate) fn parse_request(line: &str, nonce: &str) -> Callback {
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        return Callback::Ignore;
    }
    let Some(target) = parts.next() else {
        return Callback::Ignore;
    };
    let Some((path, query)) = target.split_once('?') else {
        return Callback::Ignore;
    };
    if path.trim_start_matches('/') != nonce {
        return Callback::Ignore;
    }
    let mut code = None;
    let mut error = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("code", v)) if !v.is_empty() => code = Some(v.to_string()),
            Some(("error", v)) if !v.is_empty() => error = Some(v.to_string()),
            _ => {}
        }
    }
    match (code, error) {
        (Some(c), _) => Callback::Code(c),
        (None, Some(e)) => Callback::Denied(e),
        _ => Callback::Ignore,
    }
}

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;
