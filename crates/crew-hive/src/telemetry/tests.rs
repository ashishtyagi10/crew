use super::*;
use crate::bus::{AgentId, HiveEvent};
use crate::graph::{TaskId, TaskState};

#[test]
fn spawn_then_tokens_and_cost_accumulate() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(5),
    });
    f.apply(&HiveEvent::TokenDelta {
        agent: AgentId(1),
        input: 10,
        output: 4,
    });
    f.apply(&HiveEvent::TokenDelta {
        agent: AgentId(1),
        input: 0,
        output: 6,
    });
    f.apply(&HiveEvent::CostDelta {
        agent: AgentId(1),
        micros_usd: 2500,
    });
    let a = f.get(AgentId(1)).unwrap();
    assert_eq!((a.tokens_in, a.tokens_out, a.micros_usd), (10, 10, 2500));
    assert_eq!(a.task, TaskId(5));
    assert_eq!(a.state, TaskState::Running);
}

#[test]
fn output_chunk_sets_last_nonempty_line() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(0),
    });
    f.apply(&HiveEvent::OutputChunk {
        agent: AgentId(1),
        text: "building...\nok\n".into(),
    });
    assert_eq!(f.get(AgentId(1)).unwrap().last_line, "ok");
}

#[test]
fn failed_sets_state_and_message() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(2),
        task: TaskId(1),
    });
    f.apply(&HiveEvent::Failed {
        agent: AgentId(2),
        error: "boom".into(),
    });
    let a = f.get(AgentId(2)).unwrap();
    assert_eq!(a.state, TaskState::Failed);
    assert_eq!(a.last_line, "boom");
}

#[test]
fn unknown_agent_events_ignored() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::TokenDelta {
        agent: AgentId(9),
        input: 1,
        output: 1,
    });
    assert!(f.get(AgentId(9)).is_none());
}

#[test]
fn task_state_change_updates_matching_agent() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(7),
    });
    f.apply(&HiveEvent::TaskStateChanged {
        task: TaskId(7),
        state: TaskState::Done,
    });
    assert_eq!(f.get(AgentId(1)).unwrap().state, TaskState::Done);
}

#[test]
fn totals_aggregate_across_agents() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(0),
    });
    f.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(2),
        task: TaskId(1),
    });
    f.apply(&HiveEvent::TokenDelta {
        agent: AgentId(1),
        input: 5,
        output: 7,
    });
    f.apply(&HiveEvent::CostDelta {
        agent: AgentId(2),
        micros_usd: 900,
    });
    f.apply(&HiveEvent::TaskStateChanged {
        task: TaskId(1),
        state: TaskState::Done,
    });
    let t = f.totals();
    assert_eq!(t.live, 1);
    assert_eq!(t.done, 1);
    assert_eq!(t.failed, 0);
    assert_eq!((t.tokens_in, t.tokens_out, t.micros_usd), (5, 7, 900));
}
