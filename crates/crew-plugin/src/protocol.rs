use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginCommand {
    Hello { v: u32 },
    Subscribe { channel: String },
    Send { channel: String, text: String },
}

/// One agent in a plugin's roster: its address name, a short capability role,
/// and the model it runs on (empty when unknown, e.g. an external CLI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginEvent {
    Ready {
        v: u32,
        provider: String,
        channels: Vec<String>,
    },
    /// The agents this plugin can route to (sent once after `Ready`), so the
    /// host can show a roster with model badges.
    Roster {
        agents: Vec<AgentInfo>,
    },
    /// A live status change: `agent` entered `state` (`"thinking"` while being
    /// called; `"idle"` with an empty agent when the turn ends). Drives the
    /// host's activity indicator instead of spamming the transcript.
    Activity {
        agent: String,
        state: String,
    },
    /// End-of-turn cost: agent exchanges made and approximate tokens spent.
    /// Feeds the host's running token meter.
    Stats {
        exchanges: u32,
        tokens: u64,
    },
    Message {
        channel: String,
        sender: String,
        text: String,
        /// Unix-epoch milliseconds when the message was produced ("" = unknown).
        ts: String,
        /// Optional per-message metadata for the host's log line (e.g. the
        /// reply's latency, `"4.2s"`). Absent on the wire when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        meta: String,
    },
    Error {
        message: String,
    },
    SpawnPane {
        command: String,
        args: Vec<String>,
        label: String,
    },
    SendPane {
        label: String,
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_serializes_tagged() {
        let s = serde_json::to_string(&PluginCommand::Hello { v: 1 }).unwrap();
        assert_eq!(s, r#"{"type":"hello","v":1}"#);
    }

    #[test]
    fn spawn_pane_serializes_with_type_tag() {
        let ev = PluginEvent::SpawnPane {
            command: "sh".into(),
            args: vec!["-c".into()],
            label: "a".into(),
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""type":"spawn_pane""#), "got: {s}");
    }

    #[test]
    fn send_pane_deserializes_from_json() {
        let line = r#"{"type":"send_pane","label":"a","text":"hi"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::SendPane { label, text } => {
                assert_eq!(label, "a");
                assert_eq!(text, "hi");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roster_event_roundtrips_and_defaults() {
        let line = r#"{"type":"roster","agents":[{"name":"planner","role":"planning","model":"m1"},{"name":"claude"}]}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Roster { agents } => {
                assert_eq!(agents.len(), 2);
                assert_eq!(agents[0].model, "m1");
                assert_eq!(agents[1].role, ""); // role/model default to empty
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn stats_event_roundtrips() {
        let line = r#"{"type":"stats","exchanges":3,"tokens":950}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Stats { exchanges, tokens } => {
                assert_eq!((exchanges, tokens), (3, 950));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn activity_event_roundtrips() {
        let line = r#"{"type":"activity","agent":"coder","state":"thinking"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Activity { agent, state } => {
                assert_eq!((agent.as_str(), state.as_str()), ("coder", "thinking"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn message_event_roundtrips() {
        let line = r#"{"type":"message","channel":"general","sender":"bob","text":"hi","ts":"t"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Message {
                channel,
                sender,
                text,
                ts,
                ..
            } => {
                assert_eq!(
                    (
                        channel.as_str(),
                        sender.as_str(),
                        text.as_str(),
                        ts.as_str()
                    ),
                    ("general", "bob", "hi", "t")
                );
            }
            _ => panic!("wrong variant"),
        }
    }
}
