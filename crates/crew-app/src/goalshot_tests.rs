//! Off-screen render of the `/goal` swarm pane: the HUD, the task list, the
//! bars beside it, and the three banners a run passes through.
//!
//! `chartshot` shoots the gantt half from hand-placed cells; nothing had ever
//! drawn the pane the way `SwarmPane::cells` composes it — list and timeline
//! sharing one width, an overflowing list, a cancelled run — at any size.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew goal_shot -- --ignored --nocapture`
use crew_hive::{
    AgentId, AgentKind, Fleet, HiveEvent, ModelTier, TaskGraph, TaskId, TaskSpec, TaskState,
};
use crew_render::CellView;

use crate::shotgpu_tests::shot_at;
use crate::swarm::compose::Run;
use crate::swarm::timeline::Timeline;

const NOW: u64 = 8_800;

/// A fan-out-then-join graph: task 0 first, the middle ones after it, the
/// last after all of them.
fn graph(titles: &[&str]) -> TaskGraph {
    let n = titles.len();
    let specs = titles
        .iter()
        .enumerate()
        .map(|(i, title)| TaskSpec {
            id: TaskId(i as u64),
            title: (*title).into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: match i {
                0 => vec![],
                i if i + 1 == n => (0..i as u64).map(TaskId).collect(),
                _ => vec![TaskId(0)],
            },
            prompt: format!("do: {title}"),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    TaskGraph::new(specs).expect("valid graph")
}

/// Six tasks mid-run: three done, one failed with its reason, one running
/// with its live tail, one still pending — stamped on a timeline the way the
/// pane would have seen them.
fn mid_run(fleet: &mut Fleet, tl: &mut Timeline) {
    let spawn = |f: &mut Fleet, i: u64| {
        f.apply(&HiveEvent::AgentSpawned {
            agent: AgentId(i),
            task: TaskId(i),
        })
    };
    let state = |f: &mut Fleet, i: u64, state: TaskState| {
        f.apply(&HiveEvent::TaskStateChanged {
            task: TaskId(i),
            state,
        })
    };
    spawn(fleet, 0);
    tl.observe(fleet, 0);
    state(fleet, 0, TaskState::Done);
    for i in 1..=4 {
        spawn(fleet, i);
        fleet.apply(&HiveEvent::CostDelta {
            agent: AgentId(i),
            micros_usd: 10_500,
        });
    }
    tl.observe(fleet, 1_200);
    fleet.apply(&HiveEvent::OutputChunk {
        agent: AgentId(3),
        text: "error: atlas overflow at 2048\u{d7}2048 while packing the CJK fallback".into(),
    });
    state(fleet, 3, TaskState::Failed);
    tl.observe(fleet, 3_900);
    state(fleet, 1, TaskState::Done);
    tl.observe(fleet, 5_400);
    state(fleet, 2, TaskState::Done);
    tl.observe(fleet, 6_100);
    fleet.apply(&HiveEvent::OutputChunk {
        agent: AgentId(4),
        text: "compiling widgets (14/31) \u{2014} plot::gantt".into(),
    });
    tl.observe(fleet, NOW);
}

const TITLES: [&str; 6] = [
    "read the crate",
    "map the render path",
    "map the theme path",
    "bench the atlas",
    "write the report",
    "review",
];

/// Rebuild each row as text so a shot can be read as well as looked at.
pub fn dump(cells: &[CellView], cols: u16, rows: u16) -> Vec<String> {
    (0..rows)
        .map(|r| {
            let mut line = vec![' '; cols as usize];
            for c in cells.iter().filter(|c| c.row == r) {
                if let Some(slot) = line.get_mut(c.col as usize) {
                    *slot = c.c;
                }
            }
            line.into_iter().collect::<String>().trim_end().to_string()
        })
        .collect()
}

fn run_shot(name: &str, w: u32, h: u32, titles: &[&str], cancelled: bool) -> Option<Vec<String>> {
    let graph = graph(titles);
    let mut fleet = Fleet::new();
    let mut tl = Timeline::default();
    mid_run(&mut fleet, &mut tl);
    let run = Run {
        graph: &graph,
        fleet: &fleet,
        timeline: &tl,
        cancelled,
    };
    let mut dumped = Vec::new();
    shot_at(
        &format!("goal-{name}"),
        w,
        h,
        13.0,
        "goal",
        |cols, rows, aspect| {
            let cells = run.cells(cols, rows, NOW);
            dumped = dump(&cells, cols, rows);
            eprintln!("--- goal-{name} {cols}x{rows}");
            for l in &dumped {
                eprintln!("|{l}");
            }
            (cells, run.paint(cols, rows, aspect, NOW))
        },
    )?;
    Some(dumped)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn goal_shot_running() {
    let _g = crate::app::theme_test_guard();
    let Some(rows) = run_shot("running", 900, 420, &TITLES, false) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(rows[1].contains("read the crate"), "{rows:?}");
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn goal_shot_narrow_and_tile() {
    let _g = crate::app::theme_test_guard();
    // 45 columns or fewer: the chart gives way to the names.
    run_shot("narrow", 420, 300, &TITLES, false);
    // A tile too short for the list: the overflow note.
    let many: Vec<&str> = TITLES.iter().cycle().take(14).copied().collect();
    run_shot("tile", 300, 200, &many, false);
    run_shot("overflow", 900, 220, &many, false);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn goal_shot_cancelled_and_banners() {
    let _g = crate::app::theme_test_guard();
    run_shot("cancelled", 700, 300, &TITLES, true);
    let goal = "audit every drawn widget in crew for a pixel it puts outside its own card, \
                then write the findings up as a table with one row per widget";
    for (name, text) in [
        ("planning", format!("planning: {goal}\u{2026}")),
        ("failed", format!("plan failed: {goal}")),
    ] {
        shot_at(
            &format!("goal-{name}"),
            700,
            120,
            13.0,
            "goal",
            |cols, rows, _| {
                let cells = crate::swarmpane::banner(&text, cols, rows);
                for l in dump(&cells, cols, rows) {
                    eprintln!("|{l}");
                }
                (cells, Vec::new())
            },
        );
    }
}
