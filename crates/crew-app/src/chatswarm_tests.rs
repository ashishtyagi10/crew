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
        specialty: String::new(),
        expertise: String::new(),
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
    // Two tasks, two agents to prove TokenDelta attribution via agent->task map
    p.absorb_hive_plan(vec![spec(0, "research"), spec(1, "merge")]);

    // Spawn agent 7 for task 0
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(7),
        task: TaskId(0),
    });

    // Spawn agent 8 for task 1
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(8),
        task: TaskId(1),
    });

    // Send TokenDelta for agent 7 (task 0)
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: AgentId(7),
        input: 100,
        output: 50,
    });

    // Send TokenDelta for agent 8 (task 1)
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: AgentId(8),
        input: 1,
        output: 2,
    });

    let s = p.swarm.as_ref().unwrap();
    // Task 0 should be Running, crediting agent 7's split (100 in / 50 out).
    assert_eq!(s.tasks[0].state, TaskState::Running);
    assert_eq!((s.tasks[0].tokens_in, s.tasks[0].tokens_out), (100, 50));
    // Task 1 should be Running with agent 8's split (1 in / 2 out), not summed
    // into task 0.
    assert_eq!(s.tasks[1].state, TaskState::Running);
    assert_eq!((s.tasks[1].tokens_in, s.tasks[1].tokens_out), (1, 2));
    // The run rolls both tasks' splits up for the live line's ↑in ↓out.
    assert_eq!(s.token_totals(), (101, 52));
}

#[test]
fn run_completion_clears_the_block_and_leaves_no_summary() {
    // Every task terminal → the live block is retired, but no summary record
    // is pushed: the per-agent replies already streamed into the transcript.
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
    // All terminal: block cleared, and nothing appended to the transcript.
    assert!(p.swarm.is_none());
    assert!(
        p.messages.is_empty(),
        "fold must not push a summary message: {:?}",
        p.messages.last().map(|m| &m.text)
    );
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

#[test]
fn folding_leaves_the_existing_transcript_untouched() {
    // The fold no longer pushes anything, so a full transcript neither grows
    // nor drains when a run ends.
    let mut p = pane();
    for i in 0..500 {
        p.messages.push(crate::chatlayout::Message {
            sender: "agent smith".into(),
            text: format!("m{i}"),
            ts: String::new(),
            meta: String::new(),
        });
    }
    assert_eq!(p.messages.len(), 500);
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    assert_eq!(
        p.messages.len(),
        500,
        "fold must not push or drain messages"
    );
    assert_eq!(p.messages.first().unwrap().text, "m0");
    assert_eq!(p.messages.last().unwrap().text, "m499");
}

#[test]
fn empty_plan_never_opens_a_block_or_wedges_busy() {
    let mut p = pane();
    p.absorb_hive_plan(vec![]);
    assert!(p.swarm.is_none());
    assert!(!p.is_busy());
    // An empty plan arriving while a block is open (replan) also clears it.
    p.absorb_hive_plan(vec![spec(0, "a")]);
    assert!(p.is_busy());
    p.absorb_hive_plan(vec![]);
    assert!(p.swarm.is_none());
    assert!(!p.is_busy());
}

// --- Per-task timing: `started` still drives the live line's focused-task
// ordering and elapsed readout (chatswarmview), even though the folded record
// that once reported final durations is gone.

#[test]
fn agent_spawned_sets_started_once_and_running_does_not_reset_it() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(0),
    });
    let started1 = p.swarm.as_ref().unwrap().tasks[0].started;
    assert!(started1.is_some(), "AgentSpawned must stamp `started`");

    // A later TaskStateChanged(Running) for the same task must not reset it
    // (whichever of the two arrives first wins).
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    });
    let started2 = p.swarm.as_ref().unwrap().tasks[0].started;
    assert_eq!(
        started1, started2,
        "Running must not reset an already-stamped `started`"
    );
}

#[test]
fn running_state_stamps_started_when_it_arrives_before_agent_spawned() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    });
    assert!(p.swarm.as_ref().unwrap().tasks[0].started.is_some());
}

#[test]
fn cost_deltas_are_accepted_but_no_longer_tracked() {
    // CostDelta used to accumulate per task for the folded cost summary; that
    // summary is gone, so the event is simply absorbed without panic and
    // without opening/altering task state.
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(7),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::CostDelta {
        agent: AgentId(7),
        micros_usd: 3_100,
    });
    // Still just the one running task; nothing crashed, nothing summarised.
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(s.tasks.len(), 1);
    assert_eq!(s.tasks[0].state, TaskState::Running);
}
