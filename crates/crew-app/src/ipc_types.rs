//! Wire types for the inter-pane `ask` IPC — the transport-agnostic envelope
//! shared by the `crew ask`/`crew panes` client and the running GUI. Defined
//! independently of the Unix socket so a network relay can carry the identical
//! bytes in a future federated build (see docs/vision/sentinel-network.md).
use serde::{Deserialize, Serialize};

/// Protocol version, bumped on any incompatible envelope change.
pub const PROTOCOL_V: u32 = 1;

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
    /// List the addressable panes.
    Panes { v: u32 },
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
}
