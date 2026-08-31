use super::Timeline;
use crew_hive::{AgentId, Fleet, HiveEvent, TaskId, TaskState};

fn spawn(fleet: &mut Fleet, agent: u64, task: u64) {
    fleet.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(agent),
        task: TaskId(task),
    });
}

fn finish(fleet: &mut Fleet, task: u64, state: TaskState) {
    fleet.apply(&HiveEvent::TaskStateChanged {
        task: TaskId(task),
        state,
    });
}

fn color(_: TaskState) -> (u8, u8, u8) {
    (1, 2, 3)
}

#[test]
fn a_task_is_stamped_when_it_starts_and_again_when_it_ends() {
    let mut fleet = Fleet::new();
    let mut tl = Timeline::default();
    spawn(&mut fleet, 1, 10);
    tl.observe(&fleet, 1_000);
    tl.observe(&fleet, 1_500);
    finish(&mut fleet, 10, TaskState::Done);
    tl.observe(&fleet, 2_000);
    // Later frames must not move a finished task's bar.
    tl.observe(&fleet, 9_000);
    let s = tl.spans_for(&[TaskId(10)], &fleet, 9_000, color)[0].expect("a span");
    assert_eq!((s.start_ms, s.end_ms), (1_000, 2_000));
}

#[test]
fn a_running_task_reaches_now_and_grows() {
    let mut fleet = Fleet::new();
    let mut tl = Timeline::default();
    spawn(&mut fleet, 1, 10);
    tl.observe(&fleet, 1_000);
    let a = tl.spans_for(&[TaskId(10)], &fleet, 4_000, color)[0].unwrap();
    let b = tl.spans_for(&[TaskId(10)], &fleet, 8_000, color)[0].unwrap();
    assert_eq!((a.start_ms, a.end_ms), (1_000, 4_000));
    assert_eq!(b.end_ms, 8_000, "the bar follows the clock while it runs");
}

#[test]
fn a_task_that_never_started_has_no_bar() {
    let fleet = Fleet::new();
    let tl = Timeline::default();
    assert_eq!(tl.spans_for(&[TaskId(7)], &fleet, 1_000, color)[0], None);
}

#[test]
fn the_axis_stops_growing_once_everything_has_finished() {
    let mut fleet = Fleet::new();
    let mut tl = Timeline::default();
    spawn(&mut fleet, 1, 10);
    tl.observe(&fleet, 1_000);
    // While it runs, the axis reaches now…
    assert_eq!(tl.axis(5_000), (1_000, 5_000));
    finish(&mut fleet, 10, TaskState::Done);
    tl.observe(&fleet, 6_000);
    // …and then stops, so a finished swarm's chart does not creep while
    // you are reading it.
    assert_eq!(tl.axis(60_000), (1_000, 6_000));
}

#[test]
fn the_axis_is_never_zero_width() {
    let mut fleet = Fleet::new();
    let mut tl = Timeline::default();
    spawn(&mut fleet, 1, 10);
    tl.observe(&fleet, 1_000);
    finish(&mut fleet, 10, TaskState::Done);
    tl.observe(&fleet, 1_000);
    let (t0, t1) = tl.axis(1_000);
    assert!(t1 > t0, "an instant swarm still has an axis: {t0}..{t1}");
}

#[test]
fn parallel_tasks_are_recorded_as_overlapping() {
    // The property the task list cannot show, and the reason this exists.
    let mut fleet = Fleet::new();
    let mut tl = Timeline::default();
    spawn(&mut fleet, 1, 10);
    spawn(&mut fleet, 2, 11);
    tl.observe(&fleet, 1_000);
    finish(&mut fleet, 10, TaskState::Done);
    tl.observe(&fleet, 3_000);
    finish(&mut fleet, 11, TaskState::Done);
    tl.observe(&fleet, 4_000);
    let s = tl.spans_for(&[TaskId(10), TaskId(11)], &fleet, 4_000, color);
    let (a, b) = (s[0].unwrap(), s[1].unwrap());
    assert!(a.end_ms > b.start_ms, "the two bars overlap in time");
}
