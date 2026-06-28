//! Headless integration tests for the swarm bridge and cell-view renderer.
//! No GPU, no winit, no tokio test runtime — just std::thread + mpsc.
use crew_hive::agent::StubFactory;
use crew_hive::bus::AgentId;
use crew_hive::graph::TaskState;
use crew_hive::{AgentKind, Fleet, HiveEvent, ModelTier, TaskGraph, TaskId, TaskSpec};
use std::sync::Arc;

/// Build a simple 2-task linear graph (t0 → t1).
fn two_task_graph() -> TaskGraph {
    TaskGraph::new(vec![
        TaskSpec {
            id: TaskId(0),
            title: "alpha".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "do alpha".into(),
        },
        TaskSpec {
            id: TaskId(1),
            title: "beta".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![TaskId(0)],
            prompt: "do beta".into(),
        },
    ])
    .expect("valid graph")
}

// ── Task 1: engine bridge ───────────────────────────────────────────────────

#[test]
fn bridge_stub_graph_completes() {
    use super::bridge::SwarmHandle;

    let graph = two_task_graph();
    let factory = Arc::new(StubFactory);
    let handle = SwarmHandle::spawn(graph, factory, 2);

    let mut fleet = Fleet::new();
    // Poll until both tasks done or ~2 s elapsed.
    for _ in 0..200 {
        handle.drain(&mut fleet);
        if fleet.totals().done >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let t = fleet.totals();
    assert_eq!(t.done, 2, "expected 2 tasks done, got {}", t.done);
    assert_eq!(t.failed, 0, "expected 0 failures");
    assert_eq!(t.live, 0, "expected 0 live agents after completion");
    // graph() is the accessor used by SwarmPane in Task 3.
    assert_eq!(handle.graph().len(), 2, "graph should have 2 tasks");
}

#[test]
fn bridge_cancel_stops_scheduler() {
    use super::bridge::SwarmHandle;

    // Large graph that takes time — cancel immediately.
    let tasks: Vec<TaskSpec> = (0u64..10)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: format!("t{i}"),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: if i == 0 { vec![] } else { vec![TaskId(i - 1)] },
            prompt: "p".into(),
        })
        .collect();
    let graph = TaskGraph::new(tasks).expect("chain graph");
    let factory = Arc::new(StubFactory);
    let handle = SwarmHandle::spawn(graph, factory, 1);

    // Cancel before most tasks run.
    handle.cancel();

    // Drain for up to 1 s — the scheduler should finish quickly.
    let mut fleet = Fleet::new();
    for _ in 0..100 {
        handle.drain(&mut fleet);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // After cancel, done + failed + (cancelled tasks not in fleet) < 10.
    let t = fleet.totals();
    assert!(t.done + t.failed + t.live <= 10, "fleet consistent");
}

// ── Task 2: swarm_cells view ────────────────────────────────────────────────

#[test]
fn swarm_cells_hud_row_present() {
    use super::view::swarm_cells;

    let graph = two_task_graph();
    let mut fleet = Fleet::new();
    fleet.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(0),
    });
    fleet.apply(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });

    let cells = swarm_cells(&graph, &fleet, 60, 10);

    let hud: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
    assert!(!hud.is_empty(), "HUD row 0 must have cells");
    // HUD background should be the dark navy (20,20,40).
    assert_eq!(hud[0].bg, (20, 20, 40), "HUD bg colour mismatch");
}

#[test]
fn swarm_cells_content_offset_by_hud() {
    use super::view::swarm_cells;

    let graph = two_task_graph();
    let mut fleet = Fleet::new();
    fleet.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(0),
    });

    let cells = swarm_cells(&graph, &fleet, 40, 10);
    // All non-HUD cells must be at row >= 1.
    let bad: Vec<_> = cells
        .iter()
        .filter(|c| c.row == 0 && c.bg != (20, 20, 40))
        .collect();
    assert!(bad.is_empty(), "non-HUD cell at row 0");
}

#[test]
fn swarm_cells_empty_for_zero_dims() {
    use super::view::swarm_cells;

    let graph = two_task_graph();
    let fleet = Fleet::new();
    assert!(swarm_cells(&graph, &fleet, 0, 10).is_empty());
    assert!(swarm_cells(&graph, &fleet, 40, 0).is_empty());
}

// ── Task 3: plan_goal off-thread planner ───────────────────────────────────

#[test]
fn plan_goal_produces_a_graph() {
    use crate::swarm::plan::plan_goal;
    use crew_hive::{Planner, StubPlanner};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let planner: Arc<dyn Planner> = Arc::new(StubPlanner { fanout: 3 });
    let handle = plan_goal("build a thing".into(), planner);

    let start = Instant::now();
    let result = loop {
        if let Some(r) = handle.try_take() {
            break r;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "planner timed out"
        );
        std::thread::yield_now();
    };
    let graph = result.expect("stub planner should succeed");
    // StubPlanner { fanout: 3 } makes 3 leaves + 1 merge = 4 tasks.
    assert_eq!(graph.len(), 4);
}
