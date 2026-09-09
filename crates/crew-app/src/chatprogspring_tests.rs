//! On main the bar's fill was `done × width / total`, recomputed per frame:
//! a task settling moved it a quarter of the bar in one frame.
use super::*;
use crate::chat::ChatPane;
use crate::motion::{set_level, MotionLevel};
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;
use crew_render::CellView;

const COLS: u16 = 40;

fn pane_with_swarm(n: u64) -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    let tasks = (0..n)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: format!("task-{i}"),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    p.absorb_hive_plan(tasks);
    p
}

fn settle(p: &mut ChatPane, id: u64) {
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(id),
        state: TaskState::Done,
    });
}

fn bar(p: &ChatPane, now: u64) -> Vec<CellView> {
    crate::chatprog::bar_cells(p, COLS, 5, now)
}

fn filled_cells(cells: &[CellView]) -> usize {
    cells.iter().filter(|c| c.c == '\u{2588}').count()
}

/// The filled width at +100 ms is strictly between the old and the new
/// count's widths, and exactly the new one by +COUNT_MS (420 ms).
#[test]
fn the_fill_sweeps_to_the_new_count_over_the_readout_duration() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    let mut p = pane_with_swarm(4);
    let t = 1_000;
    assert_eq!(filled_cells(&bar(&p, t)), 0, "first sight: settled at 0/4");
    settle(&mut p, 0);
    let width = bar(&p, t).len();
    let target = width / 4;
    assert!(
        target >= 2,
        "a bar {width} wide gives the sweep {target} cells"
    );
    assert_eq!(
        filled_cells(&bar(&p, t)),
        0,
        "the sweep starts where it was"
    );
    let mid = filled_cells(&bar(&p, t + 100));
    assert!(0 < mid && mid < target, "at +100: {mid} of {target}");
    assert_eq!(
        filled_cells(&bar(&p, t + crate::readout::COUNT_MS)),
        target,
        "landed at +420"
    );
    assert!(
        !live(p.swarm.as_ref().unwrap(), t + crate::readout::COUNT_MS),
        "and settled"
    );
}

#[test]
fn off_jumps_straight_to_the_new_count() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Off);
    let mut p = pane_with_swarm(4);
    let t = 1_000;
    assert_eq!(filled_cells(&bar(&p, t)), 0);
    settle(&mut p, 0);
    let width = bar(&p, t).len();
    assert_eq!(filled_cells(&bar(&p, t + 1)), width / 4, "no sweep at Off");
    set_level(MotionLevel::Full);
}

/// The leading cell glows while the fill moves — lifted toward the ink,
/// brightest at the edge — and the whole bar is the plain accent at rest.
#[test]
fn the_leading_cells_glow_only_while_the_fill_moves() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    let accent = crate::palette::accent();
    let mut p = pane_with_swarm(2);
    let t = 1_000;
    bar(&p, t);
    settle(&mut p, 0);
    bar(&p, t); // the first frame after the settle starts the sweep
    let moving = bar(&p, t + 200);
    let lit: Vec<_> = moving.iter().filter(|c| c.c == '\u{2588}').collect();
    assert!(
        lit.len() > GLOW_CELLS as usize,
        "enough cells to show a tail"
    );
    let edge = lit.last().unwrap().fg;
    let body = lit[0].fg;
    assert_ne!(edge, accent, "the leading cell glows");
    assert_eq!(body, accent, "cells behind the glow are the bar colour");
    let settled = bar(&p, t + crate::readout::COUNT_MS);
    assert!(
        settled
            .iter()
            .filter(|c| c.c == '\u{2588}')
            .all(|c| c.fg == accent),
        "at rest the bar is plain"
    );
}

#[test]
fn glow_weights_fall_off_by_cosine_from_the_edge() {
    let (bar, ink, page) = ((40, 120, 200), (250, 250, 250), (10, 10, 10));
    let edge = glow(9, 10, true, bar, ink, page);
    let back = glow(8, 10, true, bar, ink, page);
    let far = glow(6, 10, true, bar, ink, page);
    assert_ne!(edge, bar);
    assert_ne!(back, bar);
    assert_eq!(far, bar, "three cells back is the bar again");
    assert!(
        edge.0 > back.0,
        "brightest at the edge: {edge:?} > {back:?}"
    );
    assert_eq!(glow(9, 10, false, bar, ink, page), bar, "no glow at rest");
    assert_eq!(glow(12, 10, true, bar, ink, page), bar, "beyond the fill");
}
