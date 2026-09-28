//! A stop pressed while a worker waits on its model ends the run at once.
//!
//! Driven end to end: a real `ApiAgent` over a provider whose call takes five
//! seconds, the real scheduler with a counting re-planner and the default
//! pause before a second run, and the stop pressed 200 ms in. The run used to
//! wait the whole five seconds for a call nobody wanted any more, then record
//! the late answer as done.
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::apiagent::ApiFactory;
use crate::board::Blackboard;
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::planner::{PlanError, Planner};
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crate::sched::Scheduler;

/// How long the one model call takes when nobody stops it.
const CALL: Duration = Duration::from_secs(5);

/// A model that answers after [`CALL`], counting the calls made to it.
struct Slow(Arc<AtomicUsize>);

impl Provider for Slow {
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            tokio::time::sleep(CALL).await;
            Ok(Completion {
                text: "the late answer".into(),
                ..Default::default()
            })
        })
    }
}

/// Counts re-plans and offers none.
struct Replans(Arc<AtomicUsize>);

impl Planner for Replans {
    fn plan(
        &self,
        _goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(PlanError::Parse("no replacement".into())) })
    }
}

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

#[tokio::test]
async fn a_stop_while_the_model_answers_ends_the_run_at_once_as_cancelled() {
    let calls = Arc::new(AtomicUsize::new(0));
    let replans = Arc::new(AtomicUsize::new(0));
    let cancel = Arc::new(AtomicBool::new(false));
    // The default pause before a second run stays in: a retry would show in
    // the time as well as in the call count.
    let sched = Scheduler::new(
        TaskGraph::new(vec![task()]).unwrap(),
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(ApiFactory::new(Arc::new(Slow(Arc::clone(&calls))), 64)),
        1,
    )
    .with_replan("the goal", Arc::new(Replans(Arc::clone(&replans))))
    .with_cancel(Arc::clone(&cancel));
    let started = Instant::now();
    let stop = async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel.store(true, Ordering::Relaxed);
    };
    let (out, ()) = tokio::join!(sched.run(), stop);
    let took = started.elapsed();

    assert!(
        took < Duration::from_millis(1500),
        "the run returned {took:?} after starting, stop pressed at 200 ms: it \
         waited out a {CALL:?} call nobody wanted any more"
    );
    assert_eq!(out.cancelled, vec![TaskId(1)], "stopped, not {out:?}");
    assert!(out.failed.is_empty(), "a stop is not a failure: {out:?}");
    assert!(out.done.is_empty(), "nothing finished: {out:?}");
    assert_eq!(
        replans.load(Ordering::SeqCst),
        0,
        "a stop is not re-planned"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1, "a stop is not run again");
}
