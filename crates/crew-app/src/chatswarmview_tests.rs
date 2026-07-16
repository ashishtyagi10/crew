use super::*;
use crate::chat::ChatPane;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

fn pane_with_swarm(n: u64) -> ChatPane {
    let mut p = pane();
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

fn run(p: &mut ChatPane, id: u64) {
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(id),
        state: TaskState::Running,
    });
}

fn line(p: &ChatPane, cols: u16, now_ms: u64) -> String {
    let cells = block_cells(p, cols, 10, now_ms);
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == 10).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn no_swarm_no_rows() {
    let p = pane();
    assert_eq!(swarm_rows(&p, 40), 0);
    assert!(block_cells(&p, 80, 5, 0).is_empty());
}

#[test]
fn a_live_run_claims_exactly_one_row_whatever_the_plan_size() {
    // The block used to grow a row per task and cap at 8. It now says what
    // crew is doing, which is always one thing.
    assert_eq!(swarm_rows(&pane_with_swarm(1), 40), 1);
    assert_eq!(swarm_rows(&pane_with_swarm(5), 40), 1);
    assert_eq!(swarm_rows(&pane_with_swarm(20), 40), 1);
}

#[test]
fn the_line_names_the_running_task_and_counts_the_plan() {
    let mut p = pane_with_swarm(5);
    run(&mut p, 2);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-2"), "{l}");
    assert!(l.ends_with("0/5"), "{l}");
}

#[test]
fn only_the_running_task_is_named_not_the_whole_plan() {
    let mut p = pane_with_swarm(3);
    run(&mut p, 1);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-1"), "{l}");
    assert!(!l.contains("task-0"), "{l}");
    assert!(!l.contains("task-2"), "{l}");
    // One row only.
    assert!(block_cells(&p, 80, 10, 0).iter().all(|c| c.row == 10));
}

#[test]
fn the_oldest_running_task_wins_and_the_rest_are_counted() {
    // Parallel agents: naming the newest would make the line flicker as
    // tasks start and stop, so the oldest holds it.
    let mut p = pane_with_swarm(4);
    run(&mut p, 0);
    std::thread::sleep(std::time::Duration::from_millis(2));
    run(&mut p, 1);
    std::thread::sleep(std::time::Duration::from_millis(2));
    run(&mut p, 2);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-0"), "oldest should hold the line: {l}");
    assert!(l.contains("+2"), "two others run alongside it: {l}");
}

#[test]
fn a_lone_running_task_gets_no_plus_suffix() {
    let mut p = pane_with_swarm(3);
    run(&mut p, 0);
    let l = line(&p, 80, 0);
    assert!(!l.contains('+'), "{l}");
}

#[test]
fn nothing_running_shows_a_working_line_with_the_counter() {
    // The gap between the plan arriving (all Pending) and the first spawn.
    let p = pane_with_swarm(5);
    let l = line(&p, 80, 0);
    assert!(l.contains("Working"), "{l}");
    assert!(l.ends_with("0/5"), "{l}");
    assert!(!l.contains("task-"), "{l}");
}

#[test]
fn the_working_line_carries_no_elapsed() {
    // Elapsed derives from a running task's `started`; there isn't one.
    let p = pane_with_swarm(2);
    let l = line(&p, 80, 5_000);
    assert!(l.contains("Working"), "{l}");
    assert!(!l.contains('s'), "no elapsed without a running task: {l}");
}

#[test]
fn running_task_with_nonzero_now_shows_elapsed() {
    let mut p = pane_with_swarm(2);
    run(&mut p, 0);
    let l = line(&p, 80, 5_000);
    assert!(l.contains("0s"), "{l}");
}

#[test]
fn running_task_with_zero_now_shows_no_elapsed() {
    // now_ms == 0 is the test frame: deterministic, no elapsed.
    // (Don't assert `!l.contains('s')` here — the title "task-0" has one.)
    let mut p = pane_with_swarm(2);
    run(&mut p, 0);
    let l = line(&p, 80, 0);
    assert!(!l.contains("0s"), "{l}");
}

#[test]
fn the_counter_survives_a_pane_too_narrow_for_elapsed() {
    // Width rule: elapsed drops below ELAPSED_MIN_COLS, the counter never
    // does — it's the whole point of the line.
    let mut p = pane_with_swarm(5);
    run(&mut p, 0);
    let l = line(&p, ELAPSED_MIN_COLS - 1, 5_000);
    assert!(l.contains("0/5"), "{l}");
    assert!(!l.contains("0s"), "elapsed should have dropped: {l}");
}

#[test]
fn the_plus_suffix_survives_a_title_clamp() {
    // +N is the only signal that parallel work exists, so the title yields
    // columns to it rather than the reverse.
    let mut p = pane();
    let tasks = (0..3)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: "a-very-long-task-title-that-will-not-fit".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    p.absorb_hive_plan(tasks);
    run(&mut p, 0);
    run(&mut p, 1);
    let l = line(&p, 30, 0);
    assert!(l.contains("+1"), "{l}");
    assert!(l.contains("0/3"), "{l}");
}

#[test]
fn cells_never_collide_or_leave_the_pane() {
    // `reserve` and `next_start` are two expressions of the same per-column
    // budget; when they disagree the columns overlap the title.
    let mut p = pane_with_swarm(5);
    run(&mut p, 0);
    run(&mut p, 1);
    for cols in 8..=80u16 {
        let cells = block_cells(&p, cols, 10, 5_000);
        let mut seen: Vec<u16> = cells.iter().map(|c| c.col).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(
            before,
            seen.len(),
            "two glyphs share a column at cols={cols}"
        );
        assert!(
            cells.iter().all(|c| c.col < cols),
            "a cell escaped the pane at cols={cols}"
        );
    }
}

#[test]
fn wide_glyph_titles_advance_by_display_width() {
    let mut p = pane();
    let tasks = vec![TaskSpec {
        id: TaskId(0),
        title: "研究研究研究".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    }];
    p.absorb_hive_plan(tasks);
    run(&mut p, 0);
    for cols in 8..=80u16 {
        let cells = block_cells(&p, cols, 10, 5_000);
        let mut seen: Vec<u16> = cells.iter().map(|c| c.col).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "CJK title overlapped at cols={cols}");
        assert!(cells.iter().all(|c| c.col < cols), "escaped at cols={cols}");
    }
}
