//! V3 cross-host relay: carries the SAME `ipc_types` envelope over TCP to a
//! remote crew's opt-in listener, which validates a shared token and bridges
//! the request to the target instance's local Unix socket. Consent-based by
//! construction — a host is only reachable once its operator enables federation
//! and shares the token; there is no discovery of, or reach into, a host that
//! didn't opt in, and nothing here installs or propagates anything.
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ipc_types::{NoAnswer, Reply, Request};

/// The frame a federating client sends (borrowed, serialize-only — `Request`
/// isn't `Clone`, so the dialer builds this by reference).
#[derive(Serialize)]
struct RelayRequestRef<'a> {
    token: &'a str,
    instance: Option<&'a str>,
    req: &'a Request,
}

/// The frame a relay listener receives (owned): the shared `token`, the target
/// `instance` on this host, and the ipc `req` to bridge to its local socket.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub(crate) struct RelayRequest {
    pub token: String,
    pub instance: Option<String>,
    pub req: Request,
}

/// Whether a presented token is authorized. Federation must be explicitly
/// enabled: `configured` is the host's `CREW_FEDERATE_TOKEN` — `None`/empty
/// means federation is OFF and everything is rejected. The compare is
/// length-then-constant-time over the bytes so it doesn't leak the token by
/// timing.
pub(crate) fn authorized(configured: Option<&str>, presented: &str) -> bool {
    let Some(cfg) = configured.filter(|t| !t.is_empty()) else {
        return false;
    };
    if cfg.len() != presented.len() {
        return false;
    }
    cfg.bytes()
        .zip(presented.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Dial a remote crew's relay and run one request. `None` on connect/IO
/// failure (no relay listening / network error); a rejected token or an
/// unreachable pane comes back as the remote's own `Reply`, not `None`.
pub(crate) fn dial(
    host: &str,
    port: u16,
    instance: Option<&str>,
    req: &Request,
    token: &str,
) -> Option<Reply> {
    let mut stream = TcpStream::connect((host, port)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(300)));
    let frame = RelayRequestRef {
        token,
        instance,
        req,
    };
    let json = serde_json::to_string(&frame).ok()?;
    stream.write_all(json.as_bytes()).ok()?;
    stream.write_all(b"\n").ok()?;
    stream.flush().ok()?;
    let mut line = String::new();
    BufReader::new(&mut stream).read_line(&mut line).ok()?;
    serde_json::from_str(line.trim()).ok()
}

/// Serve one accepted relay connection: read the frame, reject unless its
/// token matches `configured`, else bridge the request to the target instance's
/// local socket and write the reply back. An unauthorized or unknown request
/// gets a generic `Unreachable` — never a distinct "bad token" signal.
fn serve_conn(stream: TcpStream, configured: Option<&str>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(300)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        return;
    }
    let unreachable = Reply::NoAnswer {
        reason: NoAnswer::Unreachable,
        partial: None,
    };
    let reply = match serde_json::from_str::<RelayRequest>(line.trim()) {
        Ok(f) if authorized(configured, &f.token) => {
            crate::ipc::dial_local(f.instance.as_deref(), &f.req).unwrap_or(unreachable)
        }
        _ => unreachable,
    };
    let mut s = stream;
    if let Ok(json) = serde_json::to_string(&reply) {
        let _ = s
            .write_all(json.as_bytes())
            .and_then(|_| s.write_all(b"\n"));
        let _ = s.flush();
    }
}

/// Start the relay listener IFF the operator opted in by setting a non-empty
/// `CREW_FEDERATE_TOKEN`. Absent that, nothing binds and no port is opened — a
/// host is never reachable it didn't choose to be. Binds `CREW_FEDERATE_BIND`
/// (default `0.0.0.0`, since federating is inherently cross-host) on
/// `CREW_FEDERATE_PORT` (default [`crate::askaddr::DEFAULT_RELAY_PORT`]).
pub(crate) fn maybe_spawn_listener() {
    let Some(token) = std::env::var("CREW_FEDERATE_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
    else {
        return; // federation off
    };
    let bind = std::env::var("CREW_FEDERATE_BIND").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("CREW_FEDERATE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(crate::askaddr::DEFAULT_RELAY_PORT);
    match TcpListener::bind((bind.as_str(), port)) {
        Ok(listener) => {
            eprintln!("crew federation: relay listening on {bind}:{port} (token required)");
            std::thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let token = token.clone();
                    std::thread::spawn(move || serve_conn(stream, Some(&token)));
                }
            });
        }
        Err(e) => eprintln!("crew federation: relay bind failed on {bind}:{port}: {e}"),
    }
}

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
