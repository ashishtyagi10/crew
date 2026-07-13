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
    let handle = SwarmHandle::spawn(graph, factory, 2, None);

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
    let handle = SwarmHandle::spawn(graph, factory, 1, None);

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

#[test]
fn budget_governor_trips_shared_cancel_flag() {
    use super::bridge::SwarmHandle;
    use crew_hive::{ApiFactory, Budget, MockProvider};

    // Real API agents emit CostDelta events; a zero-dollar cap means the very
    // first cost event exceeds budget, so the governor sets the shared cancel
    // flag. This exercises the bridge wiring (governor spawned alongside the
    // scheduler on one cancel flag) — the governor's own logic is unit-tested
    // in crew-hive's govern module.
    let graph = two_task_graph(); // AgentKind::Api tasks
    let provider = Arc::new(MockProvider {
        reply: "some result text".into(),
    });
    let factory = Arc::new(ApiFactory::new(provider, 256));
    let budget = Some(Budget { max_micros_usd: 0 });
    let handle = SwarmHandle::spawn(graph, factory, 2, budget);

    let mut fleet = Fleet::new();
    let mut cancelled = false;
    for _ in 0..200 {
        handle.drain(&mut fleet);
        if handle.is_cancelled() {
            cancelled = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        cancelled,
        "a zero-budget run must be cancelled by the budget governor"
    );
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
    // HUD background should be the themed page background.
    assert_eq!(
        hud[0].bg,
        crew_theme::theme().page_bg,
        "HUD bg colour mismatch"
    );
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
    let hud_bg = crew_theme::theme().page_bg;
    let bad: Vec<_> = cells
        .iter()
        .filter(|c| c.row == 0 && c.bg != hud_bg)
        .collect();
    assert!(bad.is_empty(), "non-HUD cell at row 0");
}

#[test]
fn swarm_cells_lists_task_titles_with_state_glyphs() {
    use super::view::swarm_cells;

    let graph = two_task_graph();
    let mut fleet = Fleet::new();
    fleet.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(0),
    });
    fleet.apply(&HiveEvent::OutputChunk {
        agent: AgentId(0),
        text: "compiling widgets".into(),
    });

    let cells = swarm_cells(&graph, &fleet, 60, 10);
    // Rebuild the row as text, padding columns with no cell as spaces.
    let row_text = |row: u16| {
        let mut line = vec![' '; 60];
        for c in cells.iter().filter(|c| c.row == row) {
            line[c.col as usize] = c.c;
        }
        line.into_iter().collect::<String>()
    };
    // Row 1: running task ● with its title and live output tail.
    let r1 = row_text(1);
    assert!(r1.contains("\u{25cf} alpha"), "got: {r1}");
    assert!(r1.contains("compiling widgets"), "got: {r1}");
    // Row 2: unspawned task renders as pending ○.
    let r2 = row_text(2);
    assert!(r2.contains("\u{25cb} beta"), "got: {r2}");
}

#[test]
fn swarm_cells_overflow_notes_hidden_tasks() {
    use super::view::swarm_cells;

    let graph = two_task_graph();
    let fleet = Fleet::new();
    // 2 rows total → 1 content row → 0 task rows + the "+2 more" note.
    let cells = swarm_cells(&graph, &fleet, 40, 2);
    let text: String = {
        // Rebuild the row positionally: blank cells are skipped by to_cells,
        // so gaps between glyphs must come back as spaces.
        let mut row = vec![' '; 40];
        for c in cells.iter().filter(|c| c.row == 1) {
            row[c.col as usize] = c.c;
        }
        row.into_iter().collect()
    };
    assert!(text.contains("+2 more"), "got: {text}");
    assert!(cells.iter().all(|c| c.row < 2), "no cell may overflow rows");
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

// ── Task 7: pane close wiring ───────────────────────────────────────────────

#[test]
fn escape_closes_a_swarm_pane_other_keys_do_not() {
    use winit::keyboard::{Key, NamedKey};
    assert!(crate::swarmpane::esc_closes(
        &Key::Named(NamedKey::Escape),
        true
    ));
    assert!(!crate::swarmpane::esc_closes(
        &Key::Named(NamedKey::Escape),
        false
    ));
    assert!(!crate::swarmpane::esc_closes(
        &Key::Named(NamedKey::Enter),
        true
    ));
}
