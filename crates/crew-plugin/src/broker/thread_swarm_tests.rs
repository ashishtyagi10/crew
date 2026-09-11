//! The swarm records the turn it answered — through `run_task` and the
//! injectable core — and a failed or cancelled run is not a turn.
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crew_hive::agent::{FailingFactory, StubFactory};
use crew_hive::{
    AgentFactory, AgentKind, ModelTier, PlanError, Planner, StubPlanner, TaskGraph, TaskId,
    TaskSpec,
};

use crate::broker::session::Session;
use crate::broker::swarm::{run_task, run_with_synth};
use crate::broker::testenv;
use crate::broker::thread::{lock, record};

/// A planner that hands back exactly the graph a test drew.
struct Fixed(Vec<TaskSpec>);

impl Planner for Fixed {
    fn plan(
        &self,
        _goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        let g = TaskGraph::new(self.0.clone()).expect("a valid test graph");
        Box::pin(async move { Ok(g) })
    }
}

fn spec(id: u64, title: &str, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: title.into(),
        specialty: format!("{title}::x"),
        expertise: String::new(),
    }
}

/// Two sinks off one gather — the shape that gets the lead's closing answer.
fn diamond() -> Arc<dyn Planner> {
    Arc::new(Fixed(vec![
        spec(0, "gather", &[]),
        spec(1, "left", &[0]),
        spec(2, "right", &[0]),
    ]))
}

/// The injectable core, events discarded; what it returns is the turn.
fn swarm(
    planner: Arc<dyn Planner>,
    factory: Arc<dyn AgentFactory>,
    cancelled: bool,
    synth: Option<&(dyn Fn(&str) -> Result<String, String> + 'static)>,
) -> Option<String> {
    run_with_synth(
        "compare the two",
        planner,
        factory,
        None,
        "",
        Arc::new(AtomicBool::new(cancelled)),
        None,
        synth,
        None,
        &mut |_| Ok(()),
    )
    .unwrap()
}

#[test]
fn a_failed_or_cancelled_swarm_returns_no_answer_and_records_no_turn() {
    let _env = testenv::mock("unused");
    let session = Session::new();
    let fail: HashSet<TaskId> = [TaskId(0)].into();
    let planner: Arc<dyn Planner> = Arc::new(StubPlanner { fanout: 2 });
    let failed = swarm(
        planner,
        Arc::new(FailingFactory { fail_tasks: fail }),
        false,
        None,
    );
    assert_eq!(failed, None);
    let stopped = swarm(diamond(), Arc::new(StubFactory), true, None);
    assert_eq!(stopped, None);
    record(&session.thread, "compare the two", failed);
    record(&session.thread, "compare the two", stopped);
    assert_eq!(lock(&session.thread).len(), 0);
}

#[test]
fn a_clean_swarm_with_a_closing_answer_records_that_answer() {
    let _env = testenv::mock("unused");
    let session = Session::new();
    let call = |_: &str| Ok::<String, String>("Both agree: X.".into());
    let reply = swarm(diamond(), Arc::new(StubFactory), false, Some(&call));
    assert_eq!(reply.as_deref(), Some("Both agree: X."));
    record(&session.thread, "compare the two", reply);
    let t = lock(&session.thread);
    let turn = t.turns().next().unwrap();
    assert_eq!(
        (turn.asked.as_str(), turn.answered.as_str()),
        ("compare the two", "Both agree: X.")
    );
}

#[test]
fn a_single_sink_run_through_run_task_records_the_sinks_output() {
    let _env = testenv::mock("the merged answer");
    let session = Session::new();
    run_task("build the thing", false, &session, &mut |_| Ok(())).unwrap();
    let t = lock(&session.thread);
    assert_eq!(t.len(), 1);
    let turn = t.turns().next().unwrap();
    assert_eq!(turn.asked, "build the thing");
    assert!(
        turn.answered.contains("the merged answer"),
        "{}",
        turn.answered
    );
}
