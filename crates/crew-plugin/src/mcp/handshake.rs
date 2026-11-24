//! Launching an MCP server, waiting on its replies, and agreeing a protocol
//! revision with it.
//!
//! The order is the one the 2026-07-28 stdio binding gives a client that
//! speaks both eras: send `server/discover` with crew's newest version in
//! `_meta` before anything else. A `DiscoverResult` (or an
//! `UnsupportedProtocolVersionError` that lists a version crew speaks) means a
//! modern server and no handshake at all; any other error, or silence, means a
//! legacy server, which gets the `initialize` it expects.
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::version::{self, Probe};
use super::{timeout, McpClient};
use crate::mcp::config::ServerConfig;
use crew_hive::childproc::no_console_window;

/// The probe's id, outside the counter that numbers every later request.
const PROBE_ID: u64 = 0;

/// How long the probe waits. Some legacy servers ignore a request that
/// arrives before `initialize` instead of answering it, so silence has to be
/// read as "legacy" quickly. A modern server too slow to start inside this is
/// sent `initialize` and refuses it; the host retries on later turns, which
/// only helps once the start is warm (a filled npx/uvx cache, say).
fn probe_wait() -> Duration {
    timeout().min(Duration::from_secs(3))
}

/// Why a reply never came.
pub(super) enum Silence {
    Timeout,
    Closed,
}

impl McpClient {
    /// Wait up to `within` for the response to `id`: its `result` as `Ok`,
    /// its `error` object as `Err`. Anything else on the pipe is skipped.
    pub(super) fn wait(
        &mut self,
        id: u64,
        within: Duration,
    ) -> Result<Result<Value, Value>, Silence> {
        let deadline = Instant::now() + within;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(Silence::Timeout);
            }
            match self.rx.recv_timeout(left) {
                Ok(v) if v.get("id").and_then(Value::as_u64) == Some(id) => {
                    return Ok(match v.get("error") {
                        Some(err) => Err(err.clone()),
                        None => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
                    });
                }
                Ok(_) => {} // a notification or someone else's reply
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(Silence::Closed),
            }
        }
    }

    /// Launch the server and settle which protocol revision it speaks.
    pub fn connect(cfg: &ServerConfig) -> Result<Self, String> {
        let mut c = Self::spawn(cfg)?;
        match c.probe() {
            Some(Probe::Modern(v)) => {
                c.version = v;
                Ok(c)
            }
            // Dropping `c` kills the server: a version mismatch disconnects.
            Some(Probe::Refused(why)) => Err(why),
            Some(Probe::Legacy) => c.initialize().map(|()| c),
            // A legacy server that exits on a request before `initialize`:
            // start it again and go straight to the handshake it wanted.
            None => {
                drop(c);
                let mut c = Self::spawn(cfg)?;
                c.initialize().map(|()| c)
            }
        }
    }

    /// The protocol revision agreed with this server.
    pub fn protocol_version(&self) -> &str {
        &self.version
    }

    fn spawn(cfg: &ServerConfig) -> Result<Self, String> {
        let mut child = no_console_window(&mut Command::new(&cfg.command))
            .args(&cfg.args)
            .envs(&cfg.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to launch {}: {e}", cfg.command))?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let stdin = child.stdin.take().expect("stdin was piped");
        let (tx, rx) = mpsc::channel::<Value>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let Ok(v) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                // A JSON-RPC batch (2025-03-26 asks clients to accept them)
                // is unpacked so each reply still finds its waiting request.
                let msgs = match v {
                    Value::Array(batch) => batch,
                    one => vec![one],
                };
                if msgs.into_iter().any(|m| tx.send(m).is_err()) {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            rx,
            next_id: 0,
            // Until a probe says otherwise requests go out legacy-style
            // (no `_meta`), which is what `initialize` itself must look like.
            version: version::LEGACY[0].to_string(),
        })
    }

    /// `server/discover` with crew's newest version; `None` when the server
    /// closed its pipe instead of answering.
    fn probe(&mut self) -> Option<Probe> {
        let newest = version::MODERN[0];
        let ask = json!({"jsonrpc": "2.0", "id": PROBE_ID, "method": "server/discover",
            "params": {"_meta": version::meta(newest)}});
        if self.send(&ask).is_err() {
            return None;
        }
        match self.wait(PROBE_ID, probe_wait()) {
            Ok(reply) => Some(version::classify(reply.as_ref())),
            Err(Silence::Timeout) => Some(Probe::Legacy),
            Err(Silence::Closed) => None,
        }
    }

    /// The legacy handshake, offering crew's newest legacy revision and
    /// keeping whichever one the server answers with — if crew speaks it.
    fn initialize(&mut self) -> Result<(), String> {
        let r = self.request(
            "initialize",
            json!({
                "protocolVersion": version::LEGACY[0],
                "capabilities": {},
                "clientInfo": {"name": "crew", "version": env!("CARGO_PKG_VERSION")},
            }),
        )?;
        self.version = version::accept_legacy(r.get("protocolVersion").and_then(Value::as_str))?;
        self.notify("notifications/initialized")
    }
}

#[cfg(test)]
#[path = "handshake_tests.rs"]
mod tests;
