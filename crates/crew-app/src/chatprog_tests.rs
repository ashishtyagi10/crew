use super::*;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

const COLS: u16 = 40;

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

fn settle(p: &mut ChatPane, id: u64, state: TaskState) {
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(id),
        state,
    });
}

fn filled(cells: &[CellView]) -> usize {
    cells.iter().filter(|c| c.c == '\u{2588}').count()
}

fn text(cells: &[CellView]) -> String {
    let mut v: Vec<_> = cells.iter().collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn no_run_claims_no_row_and_draws_nothing() {
    let p = pane();
    assert_eq!(progress_rows(&p, COLS), 0);
    assert!(bar_cells(&p, COLS, 5).is_empty());
}

#[test]
fn a_live_run_claims_exactly_one_row() {
    assert_eq!(progress_rows(&pane_with_swarm(4), COLS), 1);
}

#[test]
fn bar_fills_as_tasks_settle() {
    let mut p = pane_with_swarm(4);
    assert_eq!(filled(&bar_cells(&p, COLS, 5)), 0, "nothing settled yet");

    settle(&mut p, 0, TaskState::Done);
    let quarter = filled(&bar_cells(&p, COLS, 5));
    assert!(quarter > 0, "one of four settled should light some cells");

    settle(&mut p, 1, TaskState::Done);
    assert!(
        filled(&bar_cells(&p, COLS, 5)) > quarter,
        "the bar must grow as more tasks settle"
    );
}

#[test]
fn failed_and_cancelled_count_as_settled() {
    // The bar tracks "still moving", not "succeeded" — the task rows already
    // report per-task outcome. Task 2 stays pending so the run doesn't finish
    // and fold (see `a_finished_run_folds_and_releases_the_row`).
    let mut p = pane_with_swarm(3);
    settle(&mut p, 0, TaskState::Failed);
    settle(&mut p, 1, TaskState::Cancelled);
    assert!(filled(&bar_cells(&p, COLS, 5)) > 0);
}

#[test]
fn running_tasks_are_not_settled() {
    let mut p = pane_with_swarm(2);
    settle(&mut p, 0, TaskState::Running);
    assert_eq!(filled(&bar_cells(&p, COLS, 5)), 0);
}

#[test]
fn a_partly_settled_run_never_shows_a_full_bar() {
    // Integer floor: only an all-settled plan could fill the bar, and that
    // plan has already folded — so a full bar is unreachable, never a lie.
    let mut p = pane_with_swarm(3);
    settle(&mut p, 0, TaskState::Done);
    settle(&mut p, 1, TaskState::Done);
    assert!(
        bar_cells(&p, COLS, 5).iter().any(|c| c.c == '\u{2591}'),
        "2/3 must leave an unfilled cell"
    );
}

#[test]
fn a_finished_run_folds_and_releases_the_row() {
    // `absorb_hive` folds the block into the transcript once every task is
    // terminal, so the bar must vanish with it rather than linger at 100%.
    let mut p = pane_with_swarm(2);
    settle(&mut p, 0, TaskState::Done);
    assert_eq!(progress_rows(&p, COLS), 1, "still running");
    settle(&mut p, 1, TaskState::Done);
    assert_eq!(progress_rows(&p, COLS), 0, "run finished — row released");
    assert!(bar_cells(&p, COLS, 5).is_empty());
}

#[test]
fn stays_on_its_row_and_inside_the_pane() {
    let p = pane_with_swarm(5);
    let cells = bar_cells(&p, COLS, 7);
    assert!(!cells.is_empty());
    assert!(
        cells
            .iter()
            .all(|c| c.row == 7 && c.col >= INSET && c.col < COLS),
        "every cell stays on its row, inset, and within the pane width"
    );
}

#[test]
fn settled_counts_terminal_states_and_is_shared_with_the_line() {
    let mut p = pane_with_swarm(4);
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(s.settled(), (0, 4), "nothing settled yet");

    settle(&mut p, 0, TaskState::Done);
    settle(&mut p, 1, TaskState::Failed);
    settle(&mut p, 2, TaskState::Cancelled);
    let s = p.swarm.as_ref().unwrap();
    // The bar tracks "still moving", not "succeeded" — all three terminal
    // states count.
    assert_eq!(s.settled(), (3, 4));
}

#[test]
fn the_bar_no_longer_draws_a_count_label() {
    // The `2/5` moved to the status line above; the bar spans the full inset
    // width so the two surfaces don't print the same number twice.
    let p = pane_with_swarm(4);
    let t = text(&bar_cells(&p, COLS, 5));
    assert!(!t.contains('/'), "{t}");
    assert_eq!(t.chars().count(), (COLS - INSET) as usize, "{t}");
}

#[test]
fn a_pane_too_narrow_for_a_legible_bar_drops_the_row() {
    // Budget and draw must agree: if the row isn't drawn, it isn't claimed.
    let p = pane_with_swarm(4);
    for cols in 0..=8u16 {
        assert_eq!(
            progress_rows(&p, cols) == 1,
            !bar_cells(&p, cols, 5).is_empty(),
            "claimed row and drawn row disagree at cols={cols}"
        );
    }
    assert_eq!(progress_rows(&p, 4), 0, "no room for a legible bar");
}
