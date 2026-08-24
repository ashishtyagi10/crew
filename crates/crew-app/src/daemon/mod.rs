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

pub(crate) mod cli;
pub(crate) mod service;
pub(crate) mod session;

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
    /// Every way in and out. Empty until a real channel is built — the resident is reachable
    /// only from a pane today, and `crew daemon channels` says so rather than implying more.
    channels: crate::channel::Router,
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
            channels: crate::channel::Router::new(),
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

/// Answer one request. `None` for anything the daemon does not serve — the ask ops belong to the
/// GUI's endpoint, and a client that dials the wrong socket must get silence rather than a
/// confidently wrong reply.
pub(crate) fn answer(req: &Request, d: &mut Daemon) -> Option<Reply> {
    match req {
        Request::DaemonStatus { .. } => {
            let st = d.status();
            Some(Reply::Daemon {
                pid: st.pid,
                uptime_s: st.uptime_s,
                sessions: st.sessions,
                version: st.version,
            })
        }
        Request::OpenSession { label, cwd, .. } => {
            let dir = cwd.as_deref().map(std::path::Path::new);
            Some(match d.sessions.open(label, dir) {
                Ok(id) => Reply::Session { id },
                Err(e) => Reply::Failed {
                    message: format!("could not start a session: {e}"),
                },
            })
        }
        Request::Sessions { .. } => Some(Reply::Sessions {
            sessions: d
                .sessions
                .cards()
                .into_iter()
                .map(|c| crate::ipc_types::SessionCard {
                    id: c.id,
                    label: c.label,
                    cwd: c.cwd,
                    alive: c.alive,
                })
                .collect(),
        }),
        Request::Channels { .. } => Some(Reply::Channels {
            registered: d.channels.kinds().into_iter().map(str::to_string).collect(),
            ready: d
                .channels
                .ready_kinds()
                .into_iter()
                .map(str::to_string)
                .collect(),
        }),
        Request::Say { to, text, .. } => Some(match d.channels.send(to, text) {
            Ok(()) => Reply::Sent {
                id: to.clone(),
                delivered: true,
            },
            Err(e) => Reply::Failed { message: e },
        }),
        Request::SessionSend { id, line, .. } => Some(match d.sessions.send(id, line) {
            Some(delivered) => Reply::Sent {
                id: id.clone(),
                delivered,
            },
            None => Reply::Failed {
                message: format!("no such session: {id}"),
            },
        }),
        Request::SessionPoll { id, after, .. } => Some(match d.sessions.output(id, *after) {
            Some((lines, next, dropped)) => Reply::Events {
                lines,
                next,
                dropped,
            },
            None => Reply::Failed {
                message: format!("no such session: {id}"),
            },
        }),
        Request::CloseSession { id, .. } => Some(match d.sessions.close(id) {
            Some(was_alive) => Reply::Closed {
                id: id.clone(),
                was_alive,
            },
            None => Reply::Failed {
                message: format!("no such session: {id}"),
            },
        }),
        _ => None,
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
    while let Ok(incoming) = handle.rx.recv() {
        if let Some(reply) = answer(&incoming.req, &mut daemon) {
            let _ = incoming.reply.send(reply);
        }
    }
    0
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
