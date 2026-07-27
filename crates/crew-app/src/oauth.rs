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

/// The one variable this flow supplies. Shared so the gate that STARTS the
/// sign-in (`chat.rs`) and the one that stores its result (`poll.rs`) cannot
/// drift apart on a typo.
pub(crate) const OPENROUTER_KEY_VAR: &str = "OPENROUTER_API_KEY";

/// Parse the request line of the one callback we expect.
///
/// `nonce` keeps us from acting on requests that are not part of THIS flow: a
/// stray probe, another crew window's callback, a browser prefetch. It is the
/// nearest thing to an OAuth `state` a localhost callback has.
///
/// Do not read more into it than that. It is not a secret from other local
/// processes: the nonce, the port and the challenge all travel in the URL
/// handed to `open::that`, which execs a helper with that URL as an argv
/// element any local `ps` can read. PKCE stops a stolen code being REDEEMED by
/// someone else; it does not stop a local user authorizing against this same
/// challenge from their own account and posting that code here, which would
/// have crew store THEIR key. That is inherent to desktop loopback OAuth (`gh`
/// and VS Code carry the same exposure), and OpenRouter offers no `state`
/// parameter to bind the flow with, so there is no clean fix to reach for —
/// only an accurate account of what this check does and does not buy.
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
/// Wall-clock budget for ONE accepted connection, start to finish. THIS is
/// the bound that matters: `set_read_timeout` is a per-`read()` deadline and
/// [`MAX_REQUEST_BYTES`] is a byte cap, and a peer that dribbles one byte just
/// inside the read timeout satisfies both while parking the listener for
/// hours — 8192 reads at just under [`READ_TIMEOUT`] apiece is over four,
/// well past `FLOW_TIMEOUT`, holding the ephemeral port with the real callback
/// unaccepted behind it. Against a wall-clock deadline the dribble buys
/// nothing: the connection is abandoned at the deadline and the listener goes
/// straight back to waiting for the real callback.
const CONN_TIMEOUT: Duration = Duration::from_secs(5);
/// How long ONE `read()` may block. Bounds a peer that connects and says
/// nothing at all; [`CONN_TIMEOUT`] bounds everything else.
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
/// Every connection is bounded in wall-clock TIME (see [`CONN_TIMEOUT`]) and
/// in bytes (see [`MAX_REQUEST_BYTES`]) before it is even parsed: the nonce
/// check happens after the read, so it protects the flow's INTEGRITY but
/// offers no protection at all against a peer that holds the socket open or
/// feeds it one byte at a time.
fn await_callback(listener: std::net::TcpListener, nonce: &str, timeout: Duration) -> Waited {
    await_callback_within(listener, nonce, timeout, CONN_TIMEOUT)
}

/// [`await_callback`] with the per-connection budget injected, so a test can
/// pin the deadline without waiting [`CONN_TIMEOUT`] for it.
fn await_callback_within(
    listener: std::net::TcpListener,
    nonce: &str,
    timeout: Duration,
    conn_budget: Duration,
) -> Waited {
    use std::io::Write;
    if listener.set_nonblocking(true).is_err() {
        return Waited::Broken("could not watch the sign-in port");
    }
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((stream, _)) => {
                // Never past the flow's own deadline: a connection accepted in
                // the last second of the wait gets that second, not the full
                // budget.
                let until = (Instant::now() + conn_budget).min(deadline);
                let line = read_request_line(&stream, until);
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

/// Read one accepted connection's request line, abandoning it at `until`
/// however the peer behaves.
fn read_request_line(stream: &std::net::TcpStream, until: Instant) -> String {
    // Back to blocking, but never for longer than the time this connection has
    // left: `set_read_timeout` alone bounds one syscall, not the connection,
    // and re-arming it against the remaining time is what turns a per-read
    // deadline into a per-connection one.
    let _ = stream.set_nonblocking(false);
    read_bounded(stream, until, |left| {
        let _ = stream.set_read_timeout(Some(left.min(READ_TIMEOUT)));
    })
}

/// The caps on their own, over any reader — the part a test can drive without
/// a socket. Returns at most [`MAX_REQUEST_BYTES`], stops at the first
/// newline, and gives up at `until` whatever it has.
///
/// `arm` is handed the time left before every read, so a socket can re-derive
/// its `set_read_timeout` from it; anything with a deadline of its own can
/// ignore it.
fn read_bounded<R: std::io::Read>(
    mut r: R,
    until: Instant,
    mut arm: impl FnMut(Duration),
) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 256];
    while (buf.len() as u64) < MAX_REQUEST_BYTES {
        let left = until.saturating_duration_since(Instant::now());
        // Also the zero guard: `set_read_timeout(Some(ZERO))` means "block
        // forever" and is rejected outright by the OS — the one value that
        // must never reach `arm`.
        if left.is_zero() {
            break;
        }
        arm(left);
        let want = chunk.len().min((MAX_REQUEST_BYTES as usize) - buf.len());
        match r.read(&mut chunk[..want]) {
            // A signal, not the peer. Retrying is safe: the deadline check at
            // the top of the loop is what stops this spinning.
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            // The peer hung up, or the read timed out, or the socket died: all
            // three end this connection, and the caller keeps waiting for the
            // real callback.
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(nl) = buf.iter().position(|b| *b == b'\n') {
                    // Only the request line is ours; whatever headers came
                    // along in the same chunk are dropped unread.
                    buf.truncate(nl + 1);
                    break;
                }
            }
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
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
