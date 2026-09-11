use super::*;
use crate::chatswarm::SwarmStatus;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};

fn spec(id: u64) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: format!("task-{id}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    }
}

fn state(s: &mut SwarmStatus, id: u64, st: TaskState, at: u64) {
    s.apply_at(
        &HiveEvent::TaskStateChanged {
            task: TaskId(id),
            state: st,
        },
        at,
    );
}

const C: (u8, u8, u8) = (1, 2, 3);
const M: (u8, u8, u8) = (9, 9, 9);

fn picture(cells: &[(char, Color)]) -> String {
    cells.iter().map(|(c, _)| *c).collect()
}

#[test]
fn the_bar_is_dropped_below_fifty_six_columns_and_capped_at_twenty_four() {
    assert_eq!(
        bar_cols(40),
        0,
        "a narrow block keeps its titles, not its chart"
    );
    assert_eq!(bar_cols(55), 0);
    assert_eq!(bar_cols(56), 11);
    assert_eq!(bar_cols(60), 12);
    assert_eq!(bar_cols(100), 20);
    assert_eq!(bar_cols(160), 24, "wider than 120 the bar stops growing");
}

#[test]
fn the_axis_is_absent_until_a_task_starts_and_reaches_now_while_one_runs() {
    let mut s = SwarmStatus::new(vec![spec(0), spec(1)]);
    assert_eq!(axis(&s.tasks, 900), None);
    state(&mut s, 0, TaskState::Running, 100);
    assert_eq!(axis(&s.tasks, 900), Some((100, 900)));
    state(&mut s, 0, TaskState::Done, 400);
    assert_eq!(
        axis(&s.tasks, 900),
        Some((100, 400)),
        "nothing running: the axis stops at the last end"
    );
    state(&mut s, 1, TaskState::Running, 400);
    assert_eq!(axis(&s.tasks, 900), Some((100, 900)));
}

#[test]
fn a_just_started_run_has_a_one_millisecond_axis_not_a_zero_one() {
    let mut s = SwarmStatus::new(vec![spec(0)]);
    state(&mut s, 0, TaskState::Running, 500);
    assert_eq!(axis(&s.tasks, 500), Some((500, 501)));
    assert_eq!(
        axis(&s.tasks, 0),
        Some((500, 501)),
        "a clock behind the stamp"
    );
}

#[test]
fn each_span_covers_its_share_of_the_axis_and_a_running_one_reaches_now() {
    let mut s = SwarmStatus::new(vec![spec(0), spec(1), spec(2)]);
    state(&mut s, 0, TaskState::Running, 0);
    state(&mut s, 0, TaskState::Done, 500);
    state(&mut s, 1, TaskState::Running, 500);
    let ax = axis(&s.tasks, 1000).unwrap();
    assert_eq!(ax, (0, 1000));
    let a = bar(&s.tasks[0], ax, 10, 1000, C, M);
    let b = bar(&s.tasks[1], ax, 10, 1000, C, M);
    let c = bar(&s.tasks[2], ax, 10, 1000, C, M);
    assert_eq!(picture(&a), "━━━━━─────", "done: the first half");
    assert_eq!(picture(&b), "─────━━━━━", "running: from its start to now");
    assert_eq!(picture(&c), "──────────", "pending: all axis");
    assert!(a[..5].iter().all(|(_, col)| *col == C));
    assert!(a[5..].iter().all(|(_, col)| *col == M));
}

#[test]
fn an_instant_task_still_paints_one_cell() {
    let mut s = SwarmStatus::new(vec![spec(0), spec(1)]);
    state(&mut s, 0, TaskState::Running, 0);
    state(&mut s, 0, TaskState::Done, 0);
    state(&mut s, 1, TaskState::Running, 0);
    let ax = axis(&s.tasks, 1000).unwrap();
    let a = bar(&s.tasks[0], ax, 10, 1000, C, M);
    assert_eq!(picture(&a), "━─────────");
}

#[test]
fn elapsed_words_are_seconds_then_minutes() {
    assert_eq!(fmt_ms(12_400), "12s");
    assert_eq!(fmt_ms(252_000), "4m 12s");
}
