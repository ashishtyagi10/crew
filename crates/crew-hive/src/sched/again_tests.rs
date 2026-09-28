//! A failure that passes runs its task once more before anything re-plans;
//! one that lasts goes to the planner at once. Driven end to end — a real
//! `ApiAgent` over a scripted provider, the real scheduler, a counting
//! planner — so what decides is the provider's own classification
//! (`ProviderError::is_transient`), not a flag the test sets. With no tools
//! attached one agent run is exactly one provider call, so the provider's
//! call count IS the number of runs.
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::apiagent::ApiFactory;
use crate::board::Blackboard;
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::planner::{PlanError, Planner};
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crate::sched::{RunOutcome, Scheduler};

type Reply = Result<Completion, ProviderError>;

/// Answers each call with the next scripted reply, counting the calls.
struct Script {
    replies: Mutex<VecDeque<Reply>>,
    calls: Arc<AtomicUsize>,
}

impl Provider for Script {
    fn complete(&self, _req: CompletionRequest) -> Pin<Box<dyn Future<Output = Reply> + Send>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let next = self.replies.lock().unwrap().pop_front();
        Box::pin(async move { next.unwrap_or_else(|| Ok(answer("unscripted"))) })
    }
}

fn answer(text: &str) -> Completion {
    Completion {
        text: text.into(),
        ..Default::default()
    }
}

fn timed_out() -> Reply {
    Err(ProviderError::Http(
        "api.example.com went quiet \u{2014} read timed out".into(),
    ))
}

/// Counts re-plans and offers none, so nothing new runs after one and the
/// provider's count stays the failing task's alone.
struct Planners(Arc<AtomicUsize>);

impl Planner for Planners {
    fn plan(
        &self,
        _goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(PlanError::Parse("no replacement".into())) })
    }
}

/// The one task every test runs.
fn task() -> TaskSpec {
    TaskSpec {
        id: TaskId(1),
        title: "t1".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: "do the thing".into(),
        specialty: String::new(),
        expertise: String::new(),
    }
}

/// One task, run over `replies`. Returns (outcome, agent runs, re-plans).
async fn run_one(replies: Vec<Reply>) -> (RunOutcome, usize, usize) {
    let runs = Arc::new(AtomicUsize::new(0));
    let provider = Arc::new(Script {
        replies: Mutex::new(replies.into()),
        calls: Arc::clone(&runs),
    });
    let replans = Arc::new(AtomicUsize::new(0));
    let out = Scheduler::new(
        TaskGraph::new(vec![task()]).unwrap(),
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(ApiFactory::new(provider, 64)),
        1,
    )
    .with_replan("the goal", Arc::new(Planners(Arc::clone(&replans))))
    .with_retry_pause(Duration::ZERO)
    .run()
    .await;
    let (runs, replans) = (runs.load(Ordering::SeqCst), replans.load(Ordering::SeqCst));
    (out, runs, replans)
}

#[tokio::test]
async fn a_passing_failure_runs_again_and_its_success_stands_with_no_replan() {
    let (out, runs, replans) = run_one(vec![timed_out(), Ok(answer("done"))]).await;
    assert_eq!(out.done, vec![TaskId(1)], "{out:?}");
    assert!(out.failed.is_empty(), "{out:?}");
    assert_eq!(runs, 2, "the task runs exactly twice");
    assert_eq!(replans, 0, "a second run that succeeds needs no planner");
}

#[tokio::test]
async fn a_failure_that_passes_twice_runs_twice_then_replans_once() {
    let (out, runs, replans) = run_one(vec![timed_out(), timed_out()]).await;
    assert_eq!(out.failed, vec![TaskId(1)], "{out:?}");
    assert_eq!(runs, 2, "one second run, never a third");
    assert_eq!(replans, 1, "then the planner, once");
}

#[tokio::test]
async fn a_lasting_failure_goes_straight_to_the_planner() {
    let bad_key = ProviderError::Api(
        r#"{"error":{"message":"Incorrect API key provided: sk-abc***.","type":"invalid_request_error"}}"#.into(),
    );
    let (out, runs, replans) = run_one(vec![Err(bad_key), Ok(answer("never asked"))]).await;
    assert_eq!(out.failed, vec![TaskId(1)], "{out:?}");
    assert_eq!(runs, 1, "a lasting failure is never run again");
    assert_eq!(replans, 1);
}

/// A stop pressed while the task waits out its pause: the agent is not
/// running, so the second run must not start and bill after the stop.
#[tokio::test]
async fn a_stop_during_the_pause_keeps_the_first_failure() {
    use crate::agent::{Agent, AgentContext, AgentFactory};
    let runs = Arc::new(AtomicUsize::new(0));
    let provider = Arc::new(Script {
        replies: Mutex::new(vec![timed_out(), Ok(answer("after the stop"))].into()),
        calls: Arc::clone(&runs),
    });
    let agent = ApiFactory::new(provider, 64).make(&AgentKind::Api { system: None });
    let ctx = AgentContext {
        agent: crate::bus::AgentId(0),
        task: task(),
        deps: vec![],
        bus: EventBus::new(16),
        budget: crate::tools::budget::ToolBudget::solo(),
    };
    let stopped = AtomicBool::new(true);
    let result = super::run(agent.as_ref() as &dyn Agent, ctx, Duration::ZERO, &stopped).await;
    assert!(!result.success);
    assert!(
        result.output.contains("read timed out"),
        "{}",
        result.output
    );
    assert_eq!(runs.load(Ordering::SeqCst), 1, "no run after the stop");
}
