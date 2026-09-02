//! Wire types for the inter-pane `ask` IPC — the transport-agnostic envelope
//! shared by the `crew ask`/`crew panes` client and the running GUI. Defined
//! independently of the Unix socket so a network relay can carry the identical
//! bytes in a future federated build (see docs/vision/sentinel-network.md).
use serde::{Deserialize, Serialize};

pub use crate::ipc_cards::{CastAnswer, IntentCard, PaneCard, SessionCard};

/// Protocol version, bumped on any incompatible envelope change.
pub const PROTOCOL_V: u32 = 1;

/// How a broadcast ask (`crew ask --all` / `--any`) settles across the panes
/// it reaches. The fan-out and per-pane liveness are identical; only the
/// stopping rule differs — the v2 resolver widens one address to a set (see
/// docs/vision/sentinel-network.md).
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum CastMode {
    /// Ask every eligible pane; wait for them all and return every answer.
    All,
    /// Ask every eligible pane; the first real answer wins, the rest are dropped.
    Any,
}

/// A request from a client (`crew ask` / `crew panes`) to the GUI.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "op")]
pub enum Request {
    /// Ask the agent in pane `to` a `question`; `id` namespaces the sentinel.
    Ask {
        v: u32,
        from: String,
        to: String,
        question: String,
        id: String,
    },
    /// Broadcast one `question` to every eligible pane; `mode` sets the stop
    /// rule, `id` namespaces the per-pane sentinels.
    Broadcast {
        v: u32,
        from: String,
        question: String,
        id: String,
        mode: CastMode,
    },
    /// List the addressable panes.
    Panes { v: u32 },
    /// Open an agent session owned by the daemon (not by the pane that asked).
    OpenSession {
        v: u32,
        label: String,
        cwd: Option<String>,
    },
    /// List the daemon's sessions, dead ones included.
    Sessions { v: u32 },
    /// Close one session by id.
    CloseSession { v: u32, id: String },
    /// Write one line to a session's agent process.
    SessionSend { v: u32, id: String, line: String },
    /// Read a session's output from an absolute cursor. A client that died and
    /// came back polls from the cursor it last saw and is handed what it missed.
    SessionPoll { v: u32, id: String, after: usize },
    /// List the daemon's channels: every way in, and which are usable.
    Channels { v: u32 },
    /// Send one message out through a channel, addressed `kind:rest`.
    Say { v: u32, to: String, text: String },
    /// Register a standing intent: work the daemon does later, on its own.
    /// `repeat_secs` is `None` for a one-shot. The time is absolute epoch ms —
    /// "tomorrow 9am" is resolved by the client, where the user's clock is.
    Watch {
        v: u32,
        text: String,
        to: String,
        fire_ms: u64,
        repeat_secs: Option<u64>,
    },
    /// List what the daemon is waiting to do.
    Watching { v: u32 },
    /// Call one standing intent off by id.
    Unwatch { v: u32, id: String },
    /// Push one standing intent's next firing by `delay_ms` from the daemon's now.
    Snooze { v: u32, id: String, delay_ms: u64 },
    /// Ask the resident daemon what it is: pid, uptime, live session count.
    /// Served on the daemon endpoint only — the GUI's ask socket does not
    /// answer it, and the daemon does not answer the ask ops.
    DaemonStatus { v: u32 },
}

impl Request {
    /// A `DaemonStatus` stamped with the current protocol version.
    pub fn daemon_status() -> Self {
        Request::DaemonStatus { v: PROTOCOL_V }
    }
}

/// Why an ask returned without an answer.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum NoAnswer {
    /// Target went idle having produced nothing (no agent, or it ignored us).
    IdleNoEngage,
    /// Target produced output but never closed the sentinel.
    Stalled,
    /// Target was busy on its own work; we didn't disturb it.
    BusyElsewhere,
    /// No pane matched the address.
    Unreachable,
}

/// The GUI's reply to a `Request`.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "kind")]
pub enum Reply {
    Answered {
        text: String,
    },
    NoAnswer {
        reason: NoAnswer,
        partial: Option<String>,
    },
    Roster {
        panes: Vec<PaneCard>,
    },
    /// The collected outcome of a broadcast ask, one entry per pane reached.
    Cast {
        answers: Vec<CastAnswer>,
    },
    /// A session was opened; `id` is its handle.
    Session {
        id: String,
    },
    /// Every session the daemon owns.
    Sessions {
        sessions: Vec<SessionCard>,
    },
    /// A session was closed. `was_alive` distinguishes "stopped it" from "it
    /// had already died" — the caller cannot tell those apart otherwise.
    Closed {
        id: String,
        was_alive: bool,
    },
    /// Every registered channel kind, and the subset that is usable.
    Channels {
        registered: Vec<String>,
        ready: Vec<String>,
    },
    /// A line was (or was not) delivered to the session's process.
    Sent {
        id: String,
        delivered: bool,
    },
    /// A slice of a session's output. `next` is the cursor to poll from; `dropped`
    /// is how many lines fell off the front of the buffer over the session's life,
    /// so a long-absent client learns it missed some instead of quietly drawing a
    /// false history.
    Events {
        lines: Vec<String>,
        next: usize,
        dropped: usize,
    },
    /// The request was understood but could not be carried out.
    Failed {
        message: String,
    },
    /// A standing intent was registered.
    Watched {
        id: String,
        fire_ms: u64,
    },
    /// Everything the daemon is waiting to do, soonest first.
    Watchlist {
        intents: Vec<IntentCard>,
    },
    /// A standing intent was called off. `found` is false when nothing by that
    /// id was standing — a cancel that matched nothing must not report success.
    Unwatched {
        id: String,
        found: bool,
    },
    /// A standing intent was snoozed; `fire_ms` is where it landed, `None` when
    /// nothing by that id was standing.
    Snoozed {
        id: String,
        fire_ms: Option<u64>,
    },
    /// The resident daemon's status.
    Daemon {
        pid: u32,
        uptime_s: u64,
        sessions: usize,
        version: String,
    },
}

#[cfg(test)]
#[path = "ipc_types_tests.rs"]
mod tests;
