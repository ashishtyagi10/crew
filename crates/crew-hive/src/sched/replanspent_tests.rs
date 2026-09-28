//! A re-plan's cost leaves the scheduler in the run's outcome — whether its
//! plan was used or not — and a run that never re-planned spent nothing.
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::agent::FailingFactory;
use crate::board::Blackboard;
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::planner::{PlanError, Planned, Planner};
use crate::sched::{RunOutcome, Scheduler};
use crate::spent::Spent;

fn spec(id: u64, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: format!("t{id}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: String::new(),
        specialty: String::new(),
        expertise: String::new(),
    }
}

const COST: Spent = Spent {
    input: 120,
    output: 30,
    micros_usd: 900,
};

/// A re-planner that reports [`COST`] for its one call, and plans one task
/// or fails as told.
struct Billed {
    fail: bool,
}

impl Planner for Billed {
    fn plan(
        &self,
        _goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        unreachable!("the scheduler asks for the spend")
    }

    fn plan_spent(&self, _goal: &str) -> Planned {
        let plan = match self.fail {
            true => Err(PlanError::Parse("boom".into())),
            false => Ok(TaskGraph::new(vec![spec(0, &[])]).unwrap()),
        };
        Box::pin(async move { (plan, COST) })
    }
}

/// Tasks 0 and 1, with 1 failing; the re-planner as given.
async fn run(fail: &[u64], planner_fails: bool) -> RunOutcome {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[])]).unwrap();
    let factory = FailingFactory {
        fail_tasks: fail.iter().map(|f| TaskId(*f)).collect::<HashSet<_>>(),
    };
    Scheduler::new(
        g,
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(factory),
        2,
    )
    .with_retry_pause(Duration::ZERO)
    .with_replan(
        "goal",
        Arc::new(Billed {
            fail: planner_fails,
        }),
    )
    .run()
    .await
}

#[tokio::test]
async fn a_re_plan_that_ran_is_in_the_outcome() {
    assert_eq!(run(&[1], false).await.replan_spent, COST);
}

#[tokio::test]
async fn a_re_plan_that_failed_was_still_paid_for() {
    assert_eq!(run(&[1], true).await.replan_spent, COST);
}

#[tokio::test]
async fn a_run_with_no_failure_re_planned_nothing() {
    assert!(run(&[], false).await.replan_spent.is_zero());
}
