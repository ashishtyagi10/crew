use super::*;
use crate::chat::ChatPane;
use crew_hive::{AgentId, AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn spec(id: u64, title: &str) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
    }
}

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

#[test]
fn hive_plan_builds_pending_tasks() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research"), spec(1, "merge")]);
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(s.tasks.len(), 2);
    assert!(s.tasks.iter().all(|t| t.state == TaskState::Pending));
    assert!(!s.finished());
}

#[test]
fn agent_spawned_marks_running_and_token_deltas_accumulate_via_agent_map() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(7),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: AgentId(7),
        input: 100,
        output: 50,
    });
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(s.tasks[0].state, TaskState::Running);
    assert_eq!(s.tasks[0].tokens, 150);
}

#[test]
fn run_completion_folds_the_block_into_a_transcript_message() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research"), spec(1, "merge")]);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    assert!(p.swarm.is_some(), "one task still pending — not folded yet");
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(1),
        state: TaskState::Failed,
    });
    // All terminal: state cleared, record message pushed.
    assert!(p.swarm.is_none());
    let last = p.messages.last().unwrap();
    assert_eq!(last.sender, "crew");
    assert!(last.text.contains("✓ research"));
    assert!(last.text.contains("✗ merge"));
}

#[test]
fn a_second_plan_resets_the_block() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "a")]);
    p.absorb_hive_plan(vec![spec(0, "x"), spec(1, "y")]);
    assert_eq!(p.swarm.as_ref().unwrap().tasks.len(), 2);
}

#[test]
fn events_without_a_plan_are_ignored() {
    let mut p = pane();
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    }); // must not panic
    assert!(p.swarm.is_none());
}

#[test]
fn swarm_in_flight_keeps_the_pane_busy() {
    let mut p = pane();
    assert!(!p.is_busy());
    p.absorb_hive_plan(vec![spec(0, "a")]);
    assert!(p.is_busy());
}
