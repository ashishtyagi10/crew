//! Wiring for mid-swarm re-planning: `run_with` hands the scheduler a
//! replanner when one is supplied, and the replacement tasks visibly run;
//! without one (keyless/mock — `swarmconf::backend` hands `None`) a failure
//! cascade-cancels exactly as before.
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crew_hive::agent::FailingFactory;
use crew_hive::{HiveEvent, StubPlanner, TaskId};

use super::run_with;
use crate::broker::testenv;
use crate::protocol::PluginEvent;

/// Run a stub-planned swarm (leaves 0,1 + merge 2) where task 0 fails,
/// with or without a replanner. Returns every emitted event.
fn failing_run(replan: bool) -> Vec<PluginEvent> {
    let _env = testenv::mock("unused");
    let mut evs = Vec::new();
    let fail: HashSet<TaskId> = [TaskId(0)].into();
    run_with(
        "build the thing",
        Arc::new(StubPlanner { fanout: 2 }),
        Arc::new(FailingFactory { fail_tasks: fail }),
        None,
        "",
        Arc::new(AtomicBool::new(false)),
        replan.then(|| {
            let p: Arc<dyn crew_hive::Planner> = Arc::new(StubPlanner { fanout: 1 });
            p
        }),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

/// Task ids the run spawned agents for.
fn spawned(evs: &[PluginEvent]) -> Vec<TaskId> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Hive {
                event: HiveEvent::AgentSpawned { task, .. },
            } => Some(*task),
            _ => None,
        })
        .collect()
}

#[test]
fn a_replanner_reaches_the_scheduler_and_replacement_tasks_run() {
    let evs = failing_run(true);
    let ids = spawned(&evs);
    // The replacement sub-graph (StubPlanner fanout 1: one leaf + merge,
    // remapped past the old max id 2) actually executed.
    assert!(ids.contains(&TaskId(3)), "replacement leaf ran: {ids:?}");
    assert!(ids.contains(&TaskId(4)), "replacement merge ran: {ids:?}");
}

#[test]
fn without_a_replanner_a_failure_cascade_cancels_as_before() {
    let evs = failing_run(false);
    let ids = spawned(&evs);
    assert!(
        ids.iter().all(|id| id.0 <= 2),
        "no replacement tasks may appear: {ids:?}"
    );
}
