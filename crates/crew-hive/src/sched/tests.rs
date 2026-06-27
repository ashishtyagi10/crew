use super::*;
use crate::agent::{FailingFactory, StubFactory};
use crate::board::Blackboard;
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use std::collections::HashSet;
use std::sync::Arc;

fn spec(id: u64, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: format!("t{id}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: String::new(),
    }
}

#[tokio::test]
async fn runs_linear_chain_to_completion() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[1])]).unwrap();
    let board = Blackboard::new();
    let sched = Scheduler::new(
        g,
        board.clone(),
        EventBus::new(64),
        Arc::new(StubFactory),
        4,
    );
    let out = sched.run().await;
    assert_eq!(out.done, vec![TaskId(0), TaskId(1), TaskId(2)]);
    assert!(out.failed.is_empty() && out.cancelled.is_empty());
    // results landed in the board
    assert_eq!(board.result_count().await, 3);
    // dependent saw its dep's result
    assert_eq!(
        board.get_result(TaskId(2)).await.unwrap().output,
        "stub:2 deps=1"
    );
}

#[tokio::test]
async fn runs_diamond() {
    let g = TaskGraph::new(vec![
        spec(0, &[]),
        spec(1, &[0]),
        spec(2, &[0]),
        spec(3, &[1, 2]),
    ])
    .unwrap();
    let sched = Scheduler::new(
        g,
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(StubFactory),
        4,
    );
    let out = sched.run().await;
    assert_eq!(out.done, vec![TaskId(0), TaskId(1), TaskId(2), TaskId(3)]);
}

#[tokio::test]
async fn respects_concurrency_cap() {
    use crate::agent::{Agent, AgentContext, AgentFactory};
    use crate::board::TaskResult;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // An agent that tracks peak concurrency via shared atomics.
    struct Counting {
        cur: Arc<AtomicUsize>,
        max: Arc<AtomicUsize>,
    }
    impl Agent for Counting {
        fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
            let cur = self.cur.clone();
            let max = self.max.clone();
            Box::pin(async move {
                let now = cur.fetch_add(1, Ordering::SeqCst) + 1;
                max.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                cur.fetch_sub(1, Ordering::SeqCst);
                TaskResult {
                    task: ctx.task.id,
                    output: String::new(),
                    success: true,
                }
            })
        }
    }
    struct CountingFactory {
        cur: Arc<AtomicUsize>,
        max: Arc<AtomicUsize>,
    }
    impl AgentFactory for CountingFactory {
        fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
            Box::new(Counting {
                cur: self.cur.clone(),
                max: self.max.clone(),
            })
        }
    }

    let cur = Arc::new(AtomicUsize::new(0));
    let max = Arc::new(AtomicUsize::new(0));
    // 6 independent tasks, cap 2.
    let tasks: Vec<TaskSpec> = (0..6).map(|i| spec(i, &[])).collect();
    let g = TaskGraph::new(tasks).unwrap();
    let f = Arc::new(CountingFactory {
        cur: cur.clone(),
        max: max.clone(),
    });
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), f, 2)
        .run()
        .await;
    assert_eq!(out.done.len(), 6);
    assert!(
        max.load(Ordering::SeqCst) <= 2,
        "peak concurrency {} exceeded cap 2",
        max.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn failure_cascades_cancel_to_dependents() {
    // 0 fails; 1 depends on 0 -> cancelled; 2 is independent -> done.
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[])]).unwrap();
    let mut fail = HashSet::new();
    fail.insert(TaskId(0));
    let f = Arc::new(FailingFactory { fail_tasks: fail });
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), f, 4)
        .run()
        .await;
    assert_eq!(out.failed, vec![TaskId(0)]);
    assert_eq!(out.cancelled, vec![TaskId(1)]);
    assert_eq!(out.done, vec![TaskId(2)]);
}

#[tokio::test]
async fn panicking_agent_becomes_failed_not_abort() {
    use crate::agent::{Agent, AgentContext, AgentFactory};
    use crate::board::TaskResult;
    use std::future::Future;
    use std::pin::Pin;

    struct Panicker;
    impl Agent for Panicker {
        fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
            Box::pin(async move {
                if ctx.task.id == TaskId(0) {
                    panic!("boom");
                }
                TaskResult {
                    task: ctx.task.id,
                    output: String::new(),
                    success: true,
                }
            })
        }
    }
    struct PanicFactory;
    impl AgentFactory for PanicFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            Box::new(Panicker)
        }
    }
    // Task 0 panics; task 1 is independent and should still complete.
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[])]).unwrap();
    let out = Scheduler::new(
        g,
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(PanicFactory),
        4,
    )
    .run()
    .await;
    assert_eq!(out.failed, vec![TaskId(0)]);
    assert_eq!(out.done, vec![TaskId(1)]);
}

#[tokio::test]
async fn transitive_cancel() {
    // 0 fails -> 1 (dep 0) cancelled -> 2 (dep 1) cancelled.
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[1])]).unwrap();
    let mut fail = HashSet::new();
    fail.insert(TaskId(0));
    let f = Arc::new(FailingFactory { fail_tasks: fail });
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), f, 4)
        .run()
        .await;
    assert_eq!(out.failed, vec![TaskId(0)]);
    assert_eq!(out.cancelled, vec![TaskId(1), TaskId(2)]);
}

#[tokio::test]
async fn cancel_before_run_cancels_everything() {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[]), spec(2, &[])]).unwrap();
    let cancel = Arc::new(AtomicBool::new(true));
    let out = Scheduler::new(
        g,
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(StubFactory),
        4,
    )
    .with_cancel(cancel)
    .run()
    .await;
    assert!(out.done.is_empty());
    assert_eq!(out.cancelled, vec![TaskId(0), TaskId(1), TaskId(2)]);
}

#[tokio::test]
async fn cancel_mid_run_drains_inflight() {
    use crate::agent::{Agent, AgentContext, AgentFactory};
    use crate::board::TaskResult;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Agent that flips cancel after starting, then sleeps briefly and succeeds.
    struct Flip {
        cancel: Arc<AtomicBool>,
    }
    impl Agent for Flip {
        fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
            let cancel = self.cancel.clone();
            Box::pin(async move {
                cancel.store(true, Ordering::Relaxed);
                tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                TaskResult {
                    task: ctx.task.id,
                    output: String::new(),
                    success: true,
                }
            })
        }
    }
    struct FlipFactory {
        cancel: Arc<AtomicBool>,
    }
    impl AgentFactory for FlipFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            Box::new(Flip {
                cancel: self.cancel.clone(),
            })
        }
    }

    // One root (runs and flips cancel) + one dependent (should be cancelled).
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0])]).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let out = Scheduler::new(
        g,
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(FlipFactory {
            cancel: cancel.clone(),
        }),
        1,
    )
    .with_cancel(cancel)
    .run()
    .await;
    assert_eq!(out.done, vec![TaskId(0)]); // in-flight completed
    assert_eq!(out.cancelled, vec![TaskId(1)]); // unstarted dependent cancelled
}
