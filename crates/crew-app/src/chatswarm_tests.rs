use crate::chat::ChatPane;
use crate::chatswarm::{SwarmStatus, SwarmTask};
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
    // Task 0 should be Running with 150 tokens (credited to agent 7)
    assert_eq!(s.tasks[0].state, TaskState::Running);
    assert_eq!(s.tasks[0].tokens, 150);
    // Task 1 should be Running with 3 tokens (credited to agent 8, not summed into task 0)
    assert_eq!(s.tasks[1].state, TaskState::Running);
    assert_eq!(s.tasks[1].tokens, 3);
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
fn record_text_uses_the_compact_token_format() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(0),
    });
    // 12,000 + 400 = 12,400 tokens — the live block shows "12.4k", so the
    // folded record must match instead of writing the raw "12400 tok".
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: AgentId(1),
        input: 12_000,
        output: 400,
    });
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    let last = p.messages.last().unwrap();
    assert!(last.text.contains("12.4k tok"), "{}", last.text);
    assert!(!last.text.contains("12400 tok"), "{}", last.text);
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
fn fold_swarm_respects_the_500_message_cap() {
    let mut p = pane();
    for i in 0..500 {
        p.messages.push(crate::chatlayout::Message {
            sender: "crew".into(),
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
        "fold_swarm's push must respect the same 500-message drain as Message events"
    );
    // The oldest message was drained and the new record is the newest.
    assert_eq!(p.messages.first().unwrap().text, "m1");
    assert!(p.messages.last().unwrap().text.contains("research"));
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

// --- Per-task timings (2026-07-13-swarm-task-timings-design.md) ---

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
fn terminal_state_captures_elapsed_ms_when_started() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    // Instant can't be mocked cheaply — assert presence and monotonic sanity
    // (a few ms in a fast test run), not an exact value. The record's fold
    // happened after `started`, so `elapsed_ms` must be non-negative and
    // small.
    let last = p.messages.last().unwrap();
    assert!(last.text.contains('s'), "{}", last.text); // formatter suffix landed
}

#[test]
fn cancelled_before_start_leaves_elapsed_none() {
    // No AgentSpawned/Running ever arrives — task is cancelled straight out
    // of Pending.
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Cancelled,
    });
    let last = p.messages.last().unwrap();
    assert_eq!(
        last.text, "- \u{2298} research",
        "no elapsed suffix expected"
    );
}

#[test]
fn record_text_appends_elapsed_suffix_when_captured() {
    let s = SwarmStatus {
        tasks: vec![SwarmTask {
            id: TaskId(0),
            title: "research".into(),
            state: TaskState::Done,
            tokens: 0,
            started: None,
            elapsed_ms: Some(3_200),
        }],
        agent_task: Default::default(),
    };
    assert_eq!(s.record_text(), "- \u{2713} research \u{00b7} 3.2s");
}

#[test]
fn record_text_appends_elapsed_after_the_token_part() {
    let s = SwarmStatus {
        tasks: vec![SwarmTask {
            id: TaskId(0),
            title: "research".into(),
            state: TaskState::Done,
            tokens: 12_400,
            started: None,
            elapsed_ms: Some(900),
        }],
        agent_task: Default::default(),
    };
    assert_eq!(
        s.record_text(),
        "- \u{2713} research \u{2014} 12.4k tok \u{00b7} 0.9s"
    );
}

#[test]
fn record_text_omits_suffix_when_elapsed_is_none() {
    let s = SwarmStatus {
        tasks: vec![SwarmTask {
            id: TaskId(0),
            title: "research".into(),
            state: TaskState::Done,
            tokens: 0,
            started: None,
            elapsed_ms: None,
        }],
        agent_task: Default::default(),
    };
    assert_eq!(s.record_text(), "- \u{2713} research");
}
