//! OpenRouter browser sign-in: the loopback callback parser.
//!
//! This module holds the pure, I/O-free logic for interpreting the single
//! HTTP request the loopback listener receives once the user finishes
//! authorizing in their browser. Keeping it free of sockets means the
//! trickiest part of the flow — matching the nonce, picking a `code` or
//! `error` out of the query string — is fully unit-testable before any
//! listener exists.

/// What the single callback request turned out to be.
#[derive(PartialEq)]
pub(crate) enum Callback {
    Code(String),
    Denied(String),
    /// Not ours, or carries nothing usable — answer it anyway (so no browser
    /// tab hangs) and keep waiting.
    Ignore,
}

/// Hand-written so the authorization code cannot reach a log, a panic message
/// or a `{:?}` anywhere else: this type is printable (the tests format it in
/// their failure messages) but the code itself never is.
impl std::fmt::Debug for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Callback::Code(_) => f.write_str("Code(<redacted>)"),
            Callback::Denied(e) => write!(f, "Denied({e:?})"),
            Callback::Ignore => f.write_str("Ignore"),
        }
    }
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
/// How long ONE accepted connection gets to send its request line. Any local
/// process can reach this port; without this a peer that connects and sends
/// nothing would pin the worker thread in `read_line` forever — past
/// `FLOW_TIMEOUT`, holding the ephemeral port, surviving the user cancelling.
/// A silent peer now costs at most this, then is abandoned while the listener
/// goes back to waiting for the real callback.
const READ_TIMEOUT: Duration = Duration::from_secs(2);
/// Hard cap on the request line we will accumulate. `read_line` has no cap of
/// its own, so a peer streaming bytes with no newline would otherwise grow
/// crew's memory without bound. A real callback request line is a path, a
/// nonce and a code — a couple of hundred bytes; 8 KiB is already generous.
const MAX_REQUEST_BYTES: u64 = 8 * 1024;
/// Cap on remote-supplied text (an `error=` value, a server message) before it
/// reaches a pane note.
const MAX_REASON_CHARS: usize = 100;

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
            Waited::Code(code) => exchange(&code, &pkce.verifier),
            Waited::Denied(e) => OauthOutcome::Failed(e),
            Waited::TimedOut => OauthOutcome::Failed("timed out waiting for the browser".into()),
            // NOT the user being slow: the port bound but the listener could
            // not be used. Saying "timed out" here would send them off to
            // wait longer next time for something that will never work.
            Waited::Broken(why) => OauthOutcome::Failed(why.into()),
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

/// How the listener's wait ended. Distinct from [`Callback`] because "the
/// deadline passed" and "the listener itself broke" are different things to
/// tell the user, and neither is a parsed request.
#[derive(PartialEq)]
pub(crate) enum Waited {
    Code(String),
    Denied(String),
    /// `timeout` elapsed with no matching callback: the user never approved.
    TimedOut,
    /// The listener bound but could not be used. A half-working bind must not
    /// read as a slow user.
    Broken(&'static str),
}

/// Redacting, for the same reason as [`Callback`]'s.
impl std::fmt::Debug for Waited {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Waited::Code(_) => f.write_str("Code(<redacted>)"),
            Waited::Denied(e) => write!(f, "Denied({e:?})"),
            Waited::TimedOut => f.write_str("TimedOut"),
            Waited::Broken(why) => write!(f, "Broken({why:?})"),
        }
    }
}

/// Serve requests until one matches `nonce` or `timeout` elapses. Answers
/// every request (matching or not) so no browser tab hangs, but only a
/// matching one ends the wait.
///
/// Every connection is bounded in BOTH time and bytes (see [`READ_TIMEOUT`]
/// and [`MAX_REQUEST_BYTES`]) before it is even parsed: the nonce check
/// happens after the read, so it protects the flow's INTEGRITY but offers no
/// protection at all against a peer that just holds the socket open.
fn await_callback(listener: std::net::TcpListener, nonce: &str, timeout: Duration) -> Waited {
    use std::io::Write;
    if listener.set_nonblocking(true).is_err() {
        return Waited::Broken("could not watch the sign-in port");
    }
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((stream, _)) => {
                let line = read_request_line(&stream);
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
                // Anything else — junk, a probe, a peer that said nothing —
                // is answered and dropped; the wait continues for the real
                // callback until the deadline.
                match got {
                    Callback::Code(c) => return Waited::Code(c),
                    Callback::Denied(e) => return Waited::Denied(short_reason(&e)),
                    Callback::Ignore => {}
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(POLL_GAP)
            }
            Err(_) => return Waited::Broken("the sign-in port stopped accepting connections"),
        }
    }
    Waited::TimedOut
}

/// Read one accepted connection's request line, bounded in time and length.
fn read_request_line(stream: &std::net::TcpStream) -> String {
    // Back to blocking, but with a deadline of its own — a blocking read with
    // no timeout is exactly what a silent peer would exploit.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    read_bounded(stream)
}

/// The length cap on its own, over any reader — the part a test can drive
/// without a socket. Returns at most [`MAX_REQUEST_BYTES`] of input.
fn read_bounded<R: std::io::Read>(r: R) -> String {
    use std::io::BufRead;
    let mut line = String::new();
    // `take` is the cap: `read_line` would otherwise grow `line` for as long
    // as the peer keeps sending bytes without a newline.
    let _ = std::io::BufReader::new(r.take(MAX_REQUEST_BYTES)).read_line(&mut line);
    line
}

/// Clamp remote-supplied text to something a pane note can hold. The `error=`
/// value in the callback and any server message are attacker-influenced and
/// unbounded; they are not secrets, but they are not ours to print in full.
pub(crate) fn short_reason(s: &str) -> String {
    let mut out: String = s.chars().take(MAX_REASON_CHARS).collect();
    if s.chars().nth(MAX_REASON_CHARS).is_some() {
        out.push('…');
    }
    out
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
        // `to_string()` IS LOAD-BEARING — do not "improve" it to `{e:#}`.
        // `anyhow`'s `Display` prints only the OUTERMOST message, which here
        // is crew's own wording. The chain underneath can hold a serde error
        // that quotes the bytes it failed to parse — i.e. fragments of an
        // OpenRouter response that may echo the code back. The alternate
        // form walks that chain and would print them.
        Err(e) => OauthOutcome::Failed(short_reason(&e.to_string())),
    }
}

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;
