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
#[path = "timeline_tests.rs"]
mod tests;
