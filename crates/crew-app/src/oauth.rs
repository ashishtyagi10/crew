//! OpenRouter browser sign-in: the loopback callback parser.
//!
//! This module holds the pure, I/O-free logic for interpreting the single
//! HTTP request the loopback listener receives once the user finishes
//! authorizing in their browser. Keeping it free of sockets means the
//! trickiest part of the flow — matching the nonce, picking a `code` or
//! `error` out of the query string — is fully unit-testable before any
//! listener exists.

/// What the single callback request turned out to be.
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

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// How long the user gets to approve before crew stops holding the port.
const FLOW_TIMEOUT: Duration = Duration::from_secs(180);
/// How often the listener wakes to re-check the deadline.
const POLL_GAP: Duration = Duration::from_millis(100);

pub(crate) enum OauthOutcome {
    Key(String),
    /// A FLOW failure — timeout, network, non-2xx. Never a credential.
    Failed(String),
}

/// Start the browser sign-in. Returns immediately; `None` only if the loopback
/// listener could not bind, in which case the caller falls back to pasting.
///
/// EVERYTHING blocking happens on the spawned thread: this app runs
/// synchronously on the winit thread, so a blocking accept there would freeze
/// every pane. The caller polls the receiver with `try_recv`.
pub(crate) fn spawn() -> Option<Receiver<OauthOutcome>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    let pkce = crew_hive::oauth::pkce();
    let nonce = nonce();
    let callback = format!("http://127.0.0.1:{port}/{nonce}");
    let url = crew_hive::oauth::authorize_url(&callback, &pkce.challenge);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // If the browser can't be opened the user can still paste, so this is
        // not fatal on its own — the wait below simply times out.
        let _ = open::that(&url);
        let outcome = match await_callback(listener, &nonce, FLOW_TIMEOUT) {
            Callback::Code(code) => exchange(&code, &pkce.verifier),
            Callback::Denied(e) => OauthOutcome::Failed(e),
            Callback::Ignore => OauthOutcome::Failed("timed out".into()),
        };
        // A failed send means the user cancelled and dropped the receiver.
        let _ = tx.send(outcome);
    });
    Some(rx)
}

/// 32 URL-safe random characters guarding the callback path.
fn nonce() -> String {
    crew_hive::oauth::random_token(32)
}

/// Serve requests until one matches `nonce` or `timeout` elapses. Answers
/// every request (matching or not) so no browser tab hangs, but only a
/// matching one ends the wait.
fn await_callback(listener: std::net::TcpListener, nonce: &str, timeout: Duration) -> Callback {
    use std::io::{BufRead, BufReader, Write};
    if listener.set_nonblocking(true).is_err() {
        return Callback::Ignore;
    }
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                let got = parse_request(line.trim_end(), nonce);
                let body = match got {
                    Callback::Ignore => "not this crew window",
                    _ => "crew is signed in — you can close this tab.",
                };
                let mut s = &stream;
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if !matches!(got, Callback::Ignore) {
                    return got;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(POLL_GAP)
            }
            Err(_) => return Callback::Ignore,
        }
    }
    Callback::Ignore
}

/// Redeem the code on a current-thread runtime, the same shape `modelfetch`
/// uses for its one async call.
fn exchange(code: &str, verifier: &str) -> OauthOutcome {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return OauthOutcome::Failed("could not start the http runtime".into());
    };
    match rt.block_on(crew_hive::oauth::exchange_openrouter_code(code, verifier)) {
        Ok(key) => OauthOutcome::Key(key),
        Err(e) => OauthOutcome::Failed(e.to_string()),
    }
}

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;
