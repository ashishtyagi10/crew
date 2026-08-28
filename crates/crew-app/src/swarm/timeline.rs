//! When each task of a swarm ran.
//!
//! `crew-hive`'s telemetry says what state every task is in and nothing about
//! *when* it got there — deliberately: the engine has no clock and its events
//! are replayable. So the pane keeps the clock. Every drained frame, a task
//! seen Running for the first time is stamped, and one that has left Running
//! is stamped again. That is enough for a timeline, and it costs the engine
//! nothing.
//!
//! Timings are therefore observation times, not engine times: a task that
//! starts and finishes between two frames is recorded as instantaneous, which
//! is what it looked like from here.
use std::collections::HashMap;

use crew_hive::{Fleet, TaskId, TaskState};

use crate::plot::gantt::Span;

#[derive(Default)]
pub struct Timeline {
    spans: HashMap<TaskId, (u64, Option<u64>)>,
    /// The first stamp of the run — the axis' origin.
    first_ms: Option<u64>,
}

impl Timeline {
    /// Fold this frame's fleet snapshot in, at `now_ms`.
    pub fn observe(&mut self, fleet: &Fleet, now_ms: u64) {
        for a in fleet.agents() {
            let entry = self.spans.entry(a.task).or_insert_with(|| {
                // A task is only known to have started once an agent has been
                // spawned for it; pending tasks have no span at all.
                (now_ms, None)
            });
            let ended = !matches!(a.state, TaskState::Running);
            match (ended, entry.1) {
                (true, None) => entry.1 = Some(now_ms),
                // A task that goes back to running (a re-plan) reopens: the
                // bar should grow again rather than stay frozen at its first
                // ending.
                (false, Some(_)) => entry.1 = None,
                _ => {}
            }
            self.first_ms = Some(self.first_ms.map_or(entry.0, |f| f.min(entry.0)));
        }
    }

    /// The axis `(t0, t1)` at `now_ms`: from the first task seen to now, or to
    /// the last ending once everything has finished — so a completed swarm
    /// stops stretching its own chart while you look at it.
    pub fn axis(&self, now_ms: u64) -> (u64, u64) {
        let t0 = self.first_ms.unwrap_or(now_ms);
        let live = self.spans.values().any(|(_, end)| end.is_none());
        let last = self
            .spans
            .values()
            .filter_map(|(_, end)| *end)
            .max()
            .unwrap_or(now_ms);
        let t1 = if live { now_ms } else { last };
        // Never a zero-width axis: everything would land on one column.
        (t0, t1.max(t0 + 1))
    }

    /// One entry per task in `tasks` order — `None` for a task that has not
    /// started. `color_of` picks a bar's colour from its state.
    pub fn spans_for(
        &self,
        tasks: &[TaskId],
        fleet: &Fleet,
        now_ms: u64,
        color_of: impl Fn(TaskState) -> (u8, u8, u8),
    ) -> Vec<Option<Span>> {
        let state: HashMap<TaskId, TaskState> = fleet.agents().map(|a| (a.task, a.state)).collect();
        tasks
            .iter()
            .map(|id| {
                let (start, end) = self.spans.get(id)?;
                let st = state.get(id).copied().unwrap_or(TaskState::Running);
                Some(Span {
                    start_ms: *start,
                    // A running task's bar reaches *now* and grows while you
                    // watch it — the pane is already repainting for it.
                    end_ms: end.unwrap_or(now_ms),
                    color: color_of(st),
                })
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

#[cfg(test)]
mod tests {
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
}
