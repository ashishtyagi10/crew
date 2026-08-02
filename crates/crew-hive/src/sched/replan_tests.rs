//! Condition 7: a task failure triggers ONE model re-plan of the remainder —
//! completed work is never re-run, a second failure cascade-cancels as
//! before, a planner error falls back to cascade-cancel, and a malicious
//! re-plan is forced to Api/Standard (the parse_plan invariant, re-applied).
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use crate::agent::{Agent, AgentContext, AgentFactory};
use crate::board::{Blackboard, TaskResult};
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::planner::{PlanError, Planner};
use crate::sched::{RunOutcome, Scheduler};

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

/// Records `(task, is_pty, tier)` for every agent RUN, then succeeds or
/// fails per the fail set — one structure proving what actually executed.
struct Recording {
    fail: HashSet<TaskId>,
    seen: Arc<Mutex<Vec<(TaskId, bool, ModelTier)>>>,
}

impl AgentFactory for Recording {
    fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
        let (fail, seen) = (self.fail.clone(), Arc::clone(&self.seen));
        struct A {
            fail: HashSet<TaskId>,
            seen: Arc<Mutex<Vec<(TaskId, bool, ModelTier)>>>,
        }
        impl Agent for A {
            fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
                let id = ctx.task.id;
                self.seen
                    .lock()
                    .unwrap()
                    .push((id, ctx.task.agent.is_pty(), ctx.task.model));
                let ok = !self.fail.contains(&id);
                Box::pin(async move {
                    TaskResult {
                        task: id,
                        output: format!("out:{}", id.0),
                        success: ok,
                    }
                })
            }
        }
        Box::new(A { fail, seen })
    }
}

/// Returns a fixed replacement graph (or an error), recording every prompt.
struct Scripted {
    tasks: Vec<TaskSpec>,
    calls: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

impl Planner for Scripted {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        self.calls.lock().unwrap().push(goal.to_string());
        let out = if self.fail {
            Err(PlanError::Parse("boom".into()))
        } else {
            Ok(TaskGraph::new(self.tasks.clone()).unwrap())
        };
        Box::pin(async move { out })
    }
}

/// 0 succeeds, 1 fails, 2 depends on 1 — run with `replacement` as the
/// re-plan and `fail` as the failing task set. Returns
/// (outcome, planner prompts, agent runs).
async fn run_replanned(
    replacement: Vec<TaskSpec>,
    planner_fails: bool,
    fail: &[u64],
) -> (
    RunOutcome,
    Vec<String>,
    Vec<(TaskId, bool, ModelTier)>,
    Blackboard,
) {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[]), spec(2, &[1])]).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let factory = Arc::new(Recording {
        fail: fail.iter().map(|f| TaskId(*f)).collect(),
        seen: Arc::clone(&seen),
    });
    let calls = Arc::new(Mutex::new(Vec::new()));
    let planner = Arc::new(Scripted {
        tasks: replacement,
        calls: Arc::clone(&calls),
        fail: planner_fails,
    });
    let board = Blackboard::new();
    let out = Scheduler::new(g, board.clone(), EventBus::new(64), factory, 1)
        .with_replan("ship the feature", planner)
        .run()
        .await;
    let calls = calls.lock().unwrap().clone();
    let seen = seen.lock().unwrap().clone();
    (out, calls, seen, board)
}

#[tokio::test]
async fn a_failure_triggers_exactly_one_replan_and_the_replacement_runs() {
    let (out, calls, seen, board) = run_replanned(vec![spec(0, &[])], false, &[1]).await;
    assert_eq!(calls.len(), 1, "exactly one re-plan call: {calls:?}");
    // The replacement task (remapped past the old max id 2 → 3) ran and won.
    assert_eq!(out.done, vec![TaskId(0), TaskId(3)], "{out:?}");
    assert_eq!(out.failed, vec![TaskId(1)]);
    // The stale dependent of the failure was superseded, not run.
    assert!(out.cancelled.contains(&TaskId(2)), "{out:?}");
    assert!(!seen.iter().any(|(id, _, _)| *id == TaskId(2)));
    // The planner saw the goal, the completed output, and the failure.
    let p = &calls[0];
    assert!(p.contains("ship the feature"), "{p}");
    assert!(p.contains("out:0"), "completed outputs are context: {p}");
    assert!(p.contains("FAILED task"), "{p}");
    assert_eq!(board.result_count().await, 2, "one result per done task");
}

#[tokio::test]
async fn completed_tasks_are_never_rerun() {
    let (_, _, seen, _) = run_replanned(vec![spec(0, &[])], false, &[1]).await;
    let runs_of_0 = seen.iter().filter(|(id, _, _)| *id == TaskId(0)).count();
    assert_eq!(runs_of_0, 1, "task 0 must run exactly once: {seen:?}");
}

#[tokio::test]
async fn a_second_failure_cascade_cancels_without_a_second_replan() {
    // Replacement: a (fails after remap → id 3), b deps a (→ id 4).
    let (out, calls, _, _) = run_replanned(vec![spec(0, &[]), spec(1, &[0])], false, &[1, 3]).await;
    assert_eq!(calls.len(), 1, "REPLAN_CAP=1 must hold: {calls:?}");
    assert_eq!(out.failed, vec![TaskId(1), TaskId(3)], "{out:?}");
    assert!(
        out.cancelled.contains(&TaskId(4)),
        "the second failure cascades: {out:?}"
    );
}

#[tokio::test]
async fn a_planner_error_falls_back_to_cascade_cancel() {
    let (out, calls, _, _) = run_replanned(vec![], true, &[1]).await;
    assert_eq!(calls.len(), 1);
    assert_eq!(out.done, vec![TaskId(0)]);
    assert_eq!(out.failed, vec![TaskId(1)]);
    assert_eq!(
        out.cancelled,
        vec![TaskId(2)],
        "today's behavior, untouched"
    );
}

#[tokio::test]
async fn a_malicious_replan_is_forced_to_api_standard() {
    // The stub plan requests a process-executing agent on a Capable tier.
    let evil = TaskSpec {
        agent: AgentKind::Pty {
            command: "rm".into(),
            args: vec!["-rf".into(), "/".into()],
        },
        model: ModelTier::Capable,
        ..spec(0, &[])
    };
    let (out, _, seen, _) = run_replanned(vec![evil], false, &[1]).await;
    let run = seen
        .iter()
        .find(|(id, _, _)| *id == TaskId(3))
        .expect("the replacement task ran");
    assert!(!run.1, "a re-planned task must never reach a Pty agent");
    assert_eq!(run.2, ModelTier::Standard, "tier forced to Standard");
    assert!(out.done.contains(&TaskId(3)));
}
