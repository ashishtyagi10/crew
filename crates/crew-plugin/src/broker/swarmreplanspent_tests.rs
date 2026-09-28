//! A re-plan's cost reaches the swarm's turn total. The scheduler hands it
//! back in the run's outcome; no worker made the call, so the bus never
//! carried it.
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crew_hive::agent::FailingFactory;
use crew_hive::{PlanError, Planned, Planner, Spent, StubPlanner, TaskGraph, TaskId};

use super::run_with;
use crate::broker::testenv;
use crate::protocol::PluginEvent;

/// The stub's one-leaf re-plan, billed at `.0`.
struct Billed(Spent);

impl Planner for Billed {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        StubPlanner { fanout: 1 }.plan(goal)
    }

    fn plan_spent(&self, goal: &str) -> Planned {
        let (plan, spent) = (self.plan(goal), self.0);
        Box::pin(async move { (plan.await, spent) })
    }
}

/// `(tokens, tok_in, tok_out, cost)` of a stub swarm whose task 0 fails and
/// is re-planned by a planner billing `replan`.
fn total(replan: Spent) -> (u64, u64, u64, u64) {
    let _env = testenv::mock("unused");
    let mut total = None;
    let fail: HashSet<TaskId> = [TaskId(0)].into();
    run_with(
        "build the thing",
        Arc::new(StubPlanner { fanout: 2 }),
        Arc::new(FailingFactory { fail_tasks: fail }),
        None,
        "",
        Arc::new(AtomicBool::new(false)),
        Some(Arc::new(Billed(replan))),
        &mut |ev| {
            if let PluginEvent::Stats {
                agent,
                tokens,
                tok_in,
                tok_out,
                cost_microusd,
                ..
            } = ev
            {
                if agent.is_empty() {
                    total = Some((tokens, tok_in, tok_out, cost_microusd));
                }
            }
            Ok(())
        },
    )
    .unwrap();
    total.expect("the run's turn total")
}

#[test]
fn a_re_plan_s_cost_is_in_the_turn_total() {
    let (tokens, tok_in, tok_out, cost) = total(Spent::default());
    let replan = Spent {
        input: 120,
        output: 30,
        micros_usd: 900,
    };
    assert_eq!(
        total(replan),
        (tokens + 150, tok_in + 120, tok_out + 30, cost + 900)
    );
}
