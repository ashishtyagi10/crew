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
    /// called; `"idle"` with an empty agent when the turn ends). `from` names
    /// who handed the agent its work (`"user"`, a peer agent, …; may be empty),
    /// so the host can draw the live interaction, not just a busy flag.
    Activity {
        agent: String,
        state: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        from: String,
    },
    /// End-of-turn cost: agent exchanges made and approximate tokens spent.
    /// Feeds the host's running token meter. When `agent` is non-empty the
    /// event is one agent's reply stat instead — `agent` spent `ms` (and
    /// `tokens`, when the backend reports real usage) on one reply, streamed
    /// live as the hop lands — feeding the host's per-agent totals.
    Stats {
        exchanges: u32,
        tokens: u64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        agent: String,
        #[serde(default)]
        ms: u64,
        /// The reply's real prompt size in tokens — the agent's live context
        /// fill — when the backend reports usage; 0 = unknown.
        #[serde(default)]
        ctx: u64,
        /// Prompt/completion token split for the same usage `tokens` reports,
        /// and the broker-computed cost in micro-USD (0 = unpriced model).
        /// All serde-defaulted so old payloads still decode.
        #[serde(default)]
        tok_in: u64,
        #[serde(default)]
        tok_out: u64,
        #[serde(default)]
        cost_microusd: u64,
    },
    /// Mid-reply progress: `agent` has produced roughly `tokens` output
    /// tokens so far in its in-flight reply. Advisory — the end-of-hop
    /// `Stats` stays authoritative and reconciles any estimate drift.
    StatsTick {
        agent: String,
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
    /// A swarm plan landed: the full task list, so the host can open/refresh
    /// the companion graph pane. Sent once per swarm run, before execution.
    HivePlan {
        tasks: Vec<crew_hive::TaskSpec>,
    },
    /// One raw swarm telemetry event, forwarded verbatim for the host's
    /// companion graph pane. Chat-facing translations are sent separately.
    Hive {
        event: crew_hive::HiveEvent,
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
        // No agent/ms on the wire (older broker) → defaults.
        let line = r#"{"type":"stats","exchanges":3,"tokens":950}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Stats {
                exchanges,
                tokens,
                agent,
                ms,
                ..
            } => {
                assert_eq!((exchanges, tokens), (3, 950));
                assert_eq!((agent.as_str(), ms), ("", 0));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn stats_roundtrips_cost_fields_and_defaults_when_missing() {
        // Old-broker payload (no new fields) must still decode.
        let old = r#"{"type":"stats","exchanges":3,"tokens":950}"#;
        match serde_json::from_str::<PluginEvent>(old).unwrap() {
            PluginEvent::Stats {
                tok_in,
                tok_out,
                cost_microusd,
                ..
            } => {
                assert_eq!((tok_in, tok_out, cost_microusd), (0, 0, 0));
            }
            other => panic!("wrong variant: {other:?}"),
        }
        // New payload round-trips.
        let ev = PluginEvent::Stats {
            exchanges: 1,
            tokens: 950,
            agent: String::new(),
            ms: 0,
            ctx: 0,
            tok_in: 900,
            tok_out: 50,
            cost_microusd: 12_345,
        };
        let s = serde_json::to_string(&ev).unwrap();
        match serde_json::from_str::<PluginEvent>(&s).unwrap() {
            PluginEvent::Stats {
                tok_in,
                tok_out,
                cost_microusd,
                ..
            } => {
                assert_eq!((tok_in, tok_out, cost_microusd), (900, 50, 12_345));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn per_agent_stats_carry_the_reply_latency() {
        let line = r#"{"type":"stats","exchanges":0,"tokens":0,"agent":"coder","ms":4200}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Stats { agent, ms, .. } => {
                assert_eq!((agent.as_str(), ms), ("coder", 4200));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn activity_event_roundtrips() {
        // No `from` on the wire (older broker) → defaults to empty.
        let line = r#"{"type":"activity","agent":"coder","state":"thinking"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Activity { agent, state, from } => {
                assert_eq!((agent.as_str(), state.as_str()), ("coder", "thinking"));
                assert_eq!(from, "");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn activity_event_carries_who_dialed() {
        let line = r#"{"type":"activity","agent":"coder","state":"thinking","from":"planner"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Activity { agent, from, .. } => {
                assert_eq!((agent.as_str(), from.as_str()), ("coder", "planner"));
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

    #[test]
    fn stats_tick_roundtrips() {
        let ev = PluginEvent::StatsTick {
            agent: "coder".to_string(),
            tokens: 128,
        };
        let line = serde_json::to_string(&ev).unwrap();
        assert_eq!(
            line,
            r#"{"type":"stats_tick","agent":"coder","tokens":128}"#
        );
        match serde_json::from_str::<PluginEvent>(&line).unwrap() {
            PluginEvent::StatsTick { agent, tokens } => {
                assert_eq!((agent.as_str(), tokens), ("coder", 128));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn hive_events_round_trip() {
        let plan = PluginEvent::HivePlan {
            tasks: vec![crew_hive::TaskSpec {
                id: crew_hive::TaskId(0),
                title: "t".into(),
                agent: crew_hive::AgentKind::Api { system: None },
                model: crew_hive::ModelTier::Cheap,
                deps: vec![],
                prompt: "p".into(),
                specialty: String::new(),
                expertise: String::new(),
            }],
        };
        let s = serde_json::to_string(&plan).unwrap();
        assert!(s.contains("\"type\":\"hive_plan\""), "{s}");
        let ev = PluginEvent::Hive {
            event: crew_hive::HiveEvent::TaskStateChanged {
                task: crew_hive::TaskId(0),
                state: crew_hive::TaskState::Running,
            },
        };
        let s = serde_json::to_string(&ev).unwrap();
        let back: PluginEvent = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, PluginEvent::Hive { .. }));
    }
}
