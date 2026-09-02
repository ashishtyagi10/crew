use std::collections::HashSet;

use crew_hive::{
    AgentId, AgentKind, Fleet, HiveEvent, ModelTier, TaskGraph, TaskId, TaskSpec, TaskState,
};

use super::Run;
use crate::swarm::timeline::Timeline;

fn graph(n: u64) -> TaskGraph {
    TaskGraph::new(
        (0..n)
            .map(|i| TaskSpec {
                id: TaskId(i),
                title: format!("task {i}"),
                agent: AgentKind::Api { system: None },
                model: ModelTier::Cheap,
                deps: vec![],
                prompt: String::new(),
                specialty: String::new(),
                expertise: String::new(),
            })
            .collect(),
    )
    .expect("valid graph")
}

/// Every task spawned and done — a run the pane has a bar for on every row.
fn all_done(n: u64) -> (Fleet, Timeline) {
    let mut fleet = Fleet::new();
    let mut tl = Timeline::default();
    for i in 0..n {
        fleet.apply(&HiveEvent::AgentSpawned {
            agent: AgentId(i),
            task: TaskId(i),
        });
    }
    tl.observe(&fleet, 0);
    for i in 0..n {
        fleet.apply(&HiveEvent::TaskStateChanged {
            task: TaskId(i),
            state: TaskState::Done,
        });
    }
    tl.observe(&fleet, 1_000);
    (fleet, tl)
}

fn row_text(cells: &[crew_render::CellView], row: u16, cols: u16) -> String {
    let mut line = vec![' '; cols as usize];
    for c in cells.iter().filter(|c| c.row == row) {
        line[c.col as usize] = c.c;
    }
    line.into_iter().collect::<String>().trim_end().to_string()
}

#[test]
fn a_cancelled_run_keeps_its_notice_off_the_list() {
    let _g = crate::app::theme_test_guard();
    let graph = graph(14);
    let (fleet, tl) = all_done(14);
    let run = Run {
        graph: &graph,
        fleet: &fleet,
        timeline: &tl,
        cancelled: true,
    };
    let (cols, rows) = (40, 6);
    let cells = run.cells(cols, rows, 1_000);
    let mut seen = HashSet::new();
    for c in &cells {
        assert!(
            seen.insert((c.col, c.row)),
            "two cells at ({}, {}): the notice landed on the list",
            c.col,
            c.row
        );
    }
    let last = row_text(&cells, rows - 1, cols);
    assert!(last.contains("cancelled"), "last row: {last:?}");
    let note = row_text(&cells, rows - 2, cols);
    assert!(
        note.contains("more"),
        "the overflow note keeps its own row: {note:?}"
    );
}

#[test]
fn the_bars_stop_where_the_list_does() {
    let _g = crate::app::theme_test_guard();
    let graph = graph(14);
    let (fleet, tl) = all_done(14);
    let run = Run {
        graph: &graph,
        fleet: &fleet,
        timeline: &tl,
        cancelled: false,
    };
    let (cols, rows) = (100, 6);
    let cells = run.cells(cols, rows, 1_000);
    // Four tasks named (HUD + 4 + the note); the note is the last row.
    assert!(row_text(&cells, 5, cols).contains("+10 more"));
    let paint = run.paint(cols, rows, 2.0, 1_000);
    let lowest = paint.iter().map(|p| p.y + p.h).fold(0.0_f32, f32::max);
    // Bars occupy rows 1..=4; nothing may reach the note's row (5).
    assert!(
        lowest <= 5.0 + 1e-3,
        "a bar reached row {lowest}: drawn for a task the list does not name"
    );
    // And every listed task still has its bar: four lanes of ink.
    assert!(paint.len() >= 4, "{} paints", paint.len());
}
