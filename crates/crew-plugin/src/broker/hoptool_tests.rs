use std::sync::{Arc, Mutex};

use crew_hive::{AgentId, HiveEvent};

use super::hop_tooler;
use crate::PluginEvent;

fn capture() -> (
    Arc<Mutex<Vec<PluginEvent>>>,
    Arc<dyn Fn(PluginEvent) + Send + Sync>,
) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = log.clone();
    (log, Arc::new(move |e| sink.lock().unwrap().push(e)))
}

/// A call is the `Hive` event THEN the `"hive"` activity that names the
/// agent for it; a result is the `Hive` event then the bare `tool` state.
#[test]
fn a_call_and_its_result_each_become_a_hive_event_then_an_activity() {
    let (log, emit) = capture();
    let on = hop_tooler(emit, "claude".into());
    let id = AgentId::minted("claude");
    on(HiveEvent::ToolCall {
        agent: id.clone(),
        label: "Read".into(),
        args: r#"{"file_path":"/x"}"#.into(),
    });
    on(HiveEvent::ToolResult {
        agent: id.clone(),
        label: "Read".into(),
        ok: true,
        text: "fn main() {}".into(),
        ms: 12,
    });
    let evs = log.lock().unwrap();
    assert_eq!(evs.len(), 4, "{evs:?}");
    assert!(
        matches!(&evs[0], PluginEvent::Hive { event: HiveEvent::ToolCall { agent, label, .. } } if *agent == id && label == "Read")
    );
    assert!(
        matches!(&evs[1], PluginEvent::Activity { agent, state, from } if agent == "claude" && state == "tool Read" && from == "hive")
    );
    assert!(matches!(
        &evs[2],
        PluginEvent::Hive {
            event: HiveEvent::ToolResult {
                ok: true,
                ms: 12,
                ..
            }
        }
    ));
    assert!(
        matches!(&evs[3], PluginEvent::Activity { agent, state, from } if agent == "claude" && state == "tool" && from.is_empty())
    );
}

/// The minted id has the top bit set, so it can never collide with a hive
/// agent id, and it is a pure function of the name.
#[test]
fn minted_ids_are_stable_and_never_hive_ids() {
    assert_eq!(AgentId::minted("claude"), AgentId::minted("claude"));
    assert_ne!(AgentId::minted("claude"), AgentId::minted("codex"));
    assert!(AgentId::minted("claude").0 >> 63 == 1);
}
