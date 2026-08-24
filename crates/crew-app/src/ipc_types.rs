//! Wire types for the inter-pane `ask` IPC — the transport-agnostic envelope
//! shared by the `crew ask`/`crew panes` client and the running GUI. Defined
//! independently of the Unix socket so a network relay can carry the identical
//! bytes in a future federated build (see docs/vision/sentinel-network.md).
use serde::{Deserialize, Serialize};

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
    /// The resident daemon's status.
    Daemon {
        pid: u32,
        uptime_s: u64,
        sessions: usize,
        version: String,
    },
}

/// One pane's outcome within a broadcast reply. `text` is `Some` when it
/// answered; otherwise `no_answer` says why (both never set at once).
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct CastAnswer {
    pub pane: String,
    pub label: Option<String>,
    pub text: Option<String>,
    pub no_answer: Option<NoAnswer>,
}

/// One addressable pane in the `crew panes` roster.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PaneCard {
    pub id: String,
    pub label: Option<String>,
    pub kind: String,
    pub running: Option<String>,
    pub dir: Option<String>,
    pub busy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_reply_round_trip() {
        let req = Request::Ask {
            v: PROTOCOL_V,
            from: "builder".into(),
            to: "schema".into(),
            question: "which API version?".into(),
            id: "q7".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), req);

        let na = Reply::NoAnswer {
            reason: NoAnswer::IdleNoEngage,
            partial: None,
        };
        let json = serde_json::to_string(&na).unwrap();
        assert_eq!(serde_json::from_str::<Reply>(&json).unwrap(), na);

        let ans = serde_json::to_string(&Reply::Answered { text: "hi".into() }).unwrap();
        assert!(ans.contains("Answered"), "{ans}");
    }

    #[test]
    fn panes_request_parses_from_a_client_line() {
        let req: Request = serde_json::from_str(r#"{"op":"Panes","v":1}"#).unwrap();
        assert_eq!(req, Request::Panes { v: 1 });
    }

    #[test]
    fn broadcast_request_and_cast_reply_round_trip() {
        let req = Request::Broadcast {
            v: PROTOCOL_V,
            from: "builder".into(),
            question: "status?".into(),
            id: "q9".into(),
            mode: CastMode::All,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), req);

        let cast = Reply::Cast {
            answers: vec![CastAnswer {
                pane: "p1".into(),
                label: Some("schema".into()),
                text: Some("done".into()),
                no_answer: None,
            }],
        };
        let json = serde_json::to_string(&cast).unwrap();
        assert_eq!(serde_json::from_str::<Reply>(&json).unwrap(), cast);
    }
}

/// One session in a [`Reply::Sessions`] listing.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SessionCard {
    pub id: String,
    pub label: String,
    pub cwd: Option<String>,
    pub alive: bool,
}
