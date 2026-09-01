//! `crewd` — the resident brain (goal: docs/superpowers/goals/2026-08-23-jarvis-personal-assistant.md,
//! Pillar 1). The broker is a CHILD OF THE GUI today (`chatspawn::crew_broker_cmd`), so closing the
//! window ends the assistant. The daemon is the process that outlives it: it owns the session
//! registry and answers on its own local endpoint, with no display and no window anywhere in
//! its startup path.
//!
//! This module is the skeleton only — bind, serve, report. Sessions MOVE here in 1.2/1.3; until
//! they do the registry is genuinely empty and `sessions` genuinely reports 0.
//!
//! The transport is not new: [`crate::ipc`] already speaks JSON-line request/reply over a
//! Unix socket or a Windows named pipe, addressed by path. The daemon binds a DIFFERENT path
//! (`crew-daemon*.sock`) so it never collides with — or is discovered as — an ask endpoint.
use std::time::Instant;

use crate::ipc;
use crate::ipc_types::{Reply, Request};

pub(crate) mod answers;
pub(crate) mod cli;
pub(crate) mod clock;
pub(crate) mod installcli;
pub(crate) mod intent;
pub(crate) mod intentlog;
pub(crate) mod reply;
pub(crate) mod service;
pub(crate) mod session;
pub(crate) mod task;
pub(crate) mod watchcli;
pub(crate) mod wire;

pub(crate) use wire::answer;

/// The daemon's live state, flattened to the values a status reply needs. Kept separate from the
/// serving loop so the reply logic is pure and testable without binding a socket.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Status {
    pub pid: u32,
    pub uptime_s: u64,
    pub sessions: usize,
    pub version: String,
}

/// The resident itself: start time plus the session registry. Sessions are named by id; the
/// registry is empty until 1.2 moves session ownership off the pane.
pub(crate) struct Daemon {
    started: Instant,
    sessions: session::Registry,
    /// Channel addresses to agent sessions.
    bridge: task::Bridge,
    /// Every way in and out. Empty until a real channel is built — the resident is reachable
    /// only from a pane today, and `crew daemon channels` says so rather than implying more.
    channels: crate::channel::Router,
    /// What crew is waiting to do on its own clock.
    watch: intentlog::Watchlist,
}

impl Daemon {
    pub(crate) fn new() -> Self {
        Self::with_spawner(Box::new(session::ProcSpawner::broker()))
    }

    /// [`Daemon::new`] with an explicit spawner — the seam tests use to own fake processes.
    pub(crate) fn with_spawner(spawner: Box<dyn session::Spawner>) -> Self {
        Self {
            started: Instant::now(),
            sessions: session::Registry::new(spawner),
            bridge: task::Bridge::default(),
            channels: {
                let mut r = crate::channel::Router::new();
                // Registered always, ready only with a token AND an allowlist. A channel that
                // appears the moment it is configured — rather than one the user must discover
                // exists — is the difference between a setting and a secret.
                let _ = r.add(Box::new(crate::channel::telegram::Telegram::from_env()));
                r
            },
            watch: intentlog::Watchlist::at(intentlog::default_path()),
        }
    }

    /// Point the resident at a watchlist of the test's own, so a test never fires — or
    /// cancels — anything standing in the user's real one.
    #[cfg(test)]
    pub(crate) fn set_watchlist(&mut self, w: intentlog::Watchlist) {
        self.watch = w;
    }

    /// Register a channel on a running daemon. Tests use it to drive a whole round trip; a
    /// config file will use it to add whatever the user turned on.
    #[cfg(test)]
    pub(crate) fn add_channel(&mut self, c: Box<dyn crate::channel::Channel>) {
        let _ = self.channels.add(c);
    }

    /// Answer whoever messaged in, and surface anything the operator should see.
    ///
    /// The reply goes back to the address the message came from, and a send failure is printed
    /// rather than swallowed: an answer that never arrived looks exactly like a crew that is
    /// down, which is the one thing a remote channel must never look like.
    pub(crate) fn service_channels(&mut self) {
        for n in self.channels.notices() {
            println!("{n}");
        }
        let inbound = self.channels.poll();
        if inbound.is_empty() {
            return;
        }
        let snap = reply::Snapshot {
            version: crate::appregister::VERSION.to_string(),
            uptime_s: self.started.elapsed().as_secs(),
            sessions: self.sessions.cards(),
        };
        // Collected before sending: routing a task borrows the session registry, and the
        // channels are borrowed to answer on.
        let mut outgoing: Vec<(String, String)> = Vec::new();
        for msg in inbound {
            let answer = match reply::respond(&msg.text, &snap) {
                Some(a) => a,
                // Not a question about the resident — hand it to an agent.
                None => match self
                    .bridge
                    .dispatch(&mut self.sessions, &msg.from, &msg.text)
                {
                    Ok(ack) => ack.to_string(),
                    Err(e) => e,
                },
            };
            outgoing.push((msg.from, answer));
        }
        for (addr, text) in outgoing {
            if let Err(e) = self.channels.send(&addr, &text) {
                println!("could not answer {addr}: {e}");
            }
        }
    }

    /// Forward anything the agent sessions have said back to whoever asked.
    pub(crate) fn deliver_replies(&mut self) {
        for (addr, text) in self.bridge.collect(&self.sessions) {
            if let Err(e) = self.channels.send(&addr, &text) {
                println!("could not deliver to {addr}: {e}");
            }
        }
    }

    pub(crate) fn status(&mut self) -> Status {
        Status {
            pid: std::process::id(),
            uptime_s: self.started.elapsed().as_secs(),
            sessions: self.sessions.len(),
            version: crate::appregister::VERSION.to_string(),
        }
    }
}

/// Ask whoever holds the daemon endpoint for its status. `None` = nothing listening (no daemon,
/// or a stale socket file no process is bound to).
pub(crate) fn probe(instance: Option<&str>) -> Option<Status> {
    probe_at(&ipc::daemon_socket_path_for(instance))
}

/// [`probe`] against an explicit endpoint path (the test seam, and the shape `run_at` needs).
pub(crate) fn probe_at(path: &std::path::Path) -> Option<Status> {
    match ipc::exchange_at(path, &Request::daemon_status())? {
        Reply::Daemon {
            pid,
            uptime_s,
            sessions,
            version,
        } => Some(Status {
            pid,
            uptime_s,
            sessions,
            version,
        }),
        _ => None,
    }
}

/// Send one request to the resident's endpoint. `None` = nothing listening.
pub(crate) fn request(instance: Option<&str>, req: &Request) -> Option<Reply> {
    ipc::exchange_at(&ipc::daemon_socket_path_for(instance), req)
}

/// Serve until killed. Refuses to start when a daemon already answers on this endpoint —
/// `ipc::spawn_at` reclaims a stale socket unconditionally, which would silently HIJACK a live
/// daemon's path and leave two residents fighting over one address.
pub(crate) fn run(instance: Option<&str>) -> i32 {
    run_at(ipc::daemon_socket_path_for(instance))
}

/// [`run`] against an explicit endpoint path, so a test can stand a real resident up on a
/// temporary socket instead of the user's live one.
pub(crate) fn run_at(path: std::path::PathBuf) -> i32 {
    if let Some(live) = probe_at(&path) {
        eprintln!(
            "crew daemon: already running (pid {}, up {}s)",
            live.pid, live.uptime_s
        );
        return 1;
    }
    let handle = match ipc::spawn_at(path.clone()) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("crew daemon: could not bind {}: {e}", path.display());
            return 1;
        }
    };
    let mut daemon = Daemon::new();
    println!(
        "crew daemon {} listening on {} (pid {})",
        crate::appregister::VERSION,
        path.display(),
        std::process::id()
    );
    // A timeout rather than a plain recv: between requests the resident has its own work —
    // draining channels, answering whoever messaged it. A daemon that only wakes when asked is
    // not resident, it is a server.
    loop {
        match handle
            .rx
            .recv_timeout(std::time::Duration::from_millis(250))
        {
            Ok(incoming) => {
                if let Some(reply) = answer(&incoming.req, &mut daemon) {
                    let _ = incoming.reply.send(reply);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        daemon.service_channels();
        daemon.service_intents(crate::chattime::unix_now_ms());
        daemon.deliver_replies();
    }
    0
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
