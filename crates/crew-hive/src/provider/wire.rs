//! What a transport failure SAYS.
//!
//! `reqwest::Error`'s `Display` is its KIND and nothing else. Every failure
//! while reading a response body — a timeout, a dropped connection, a
//! truncated frame — prints the same six words: `error decoding response
//! body`. The real cause lives in `source()`, which `to_string()` never
//! walks, and the URL is attached only to errors raised before the response
//! headers, so a body error names neither the host nor the reason.
//!
//! That is how eight agents in one fan-out reported eight identical
//! `[error] http error: error decoding response body` lines for what was one
//! 120-second read timeout: nothing on screen said "timeout", so nothing on
//! screen said which knob to turn. This module turns one of those errors
//! into a sentence that names the cause and the endpoint it happened on.

/// The host of `endpoint`, or the whole string when it does not parse — a
/// diagnostic never gets to fail.
fn host_of(endpoint: &str) -> &str {
    let rest = endpoint
        .split_once("://")
        .map_or(endpoint, |(_scheme, rest)| rest);
    rest.split('/').next().unwrap_or(rest)
}

/// The innermost `source()` of `e` — the layer that actually failed
/// (`operation timed out`, `connection closed before message completed`, …).
fn root_cause(e: &reqwest::Error) -> Option<String> {
    let mut src: &(dyn std::error::Error + 'static) = std::error::Error::source(e)?;
    while let Some(next) = std::error::Error::source(src) {
        src = next;
    }
    Some(src.to_string())
}

/// One transport failure against `endpoint`, in words: what went wrong, where,
/// and the underlying layer's own message in parentheses when it adds
/// anything.
pub(super) fn wire_error(e: &reqwest::Error, endpoint: &str) -> String {
    let host = host_of(endpoint);
    let what = if e.is_timeout() {
        // Both the wait for the first byte and a mid-stream gap land here:
        // the client's timeout is a SILENCE budget (`provider::http_client`).
        format!("{host} went quiet — read timed out")
    } else if e.is_connect() {
        format!("could not connect to {host}")
    } else if e.is_body() || e.is_decode() {
        format!("{host} dropped the response mid-stream")
    } else {
        format!("request to {host} failed")
    };
    match root_cause(e) {
        Some(cause) if !cause.is_empty() && !what.contains(&cause) => format!("{what} ({cause})"),
        _ => what,
    }
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
