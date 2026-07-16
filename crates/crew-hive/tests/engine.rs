//! End-to-end: build a graph, run it through the scheduler with stub agents,
//! and drive a telemetry Fleet from the bus — using ONLY the public API.
use crew_hive::{
    AgentKind, Blackboard, EventBus, Fleet, ModelTier, Scheduler, StubAgent, TaskGraph, TaskId,
    TaskSpec, TaskState,
};
use std::sync::Arc;

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

// A factory exported for downstream use: build via the public StubAgent.
struct Stubs;
impl crew_hive::AgentFactory for Stubs {
    fn make(&self, _k: &AgentKind) -> Box<dyn crew_hive::Agent> {
        Box::new(StubAgent {
            fail_ids: std::collections::HashSet::new(),
        })
    }
}

#[tokio::test]
async fn end_to_end_fan_out_fan_in() {
    let g = TaskGraph::new(vec![
        spec(0, &[]),
        spec(1, &[0]),
        spec(2, &[0]),
        spec(3, &[1, 2]),
    ])
    .unwrap();
    let board = Blackboard::new();
    let bus = EventBus::new(256);

    // Drive telemetry from the bus concurrently.
    let mut rx = bus.subscribe();
    let collector = tokio::spawn(async move {
        let mut fleet = Fleet::new();
        while let Ok(ev) = rx.recv().await {
            fleet.apply(&ev);
        }
        fleet
    });

    let out = Scheduler::new(g, board.clone(), bus.clone(), Arc::new(Stubs), 8)
        .run()
        .await;
    drop(bus); // close the channel so the collector finishes
    let fleet = collector.await.unwrap();

    assert_eq!(out.done, vec![TaskId(0), TaskId(1), TaskId(2), TaskId(3)]);
    assert_eq!(board.result_count().await, 4);
    // every task reached Done in telemetry
    let totals = fleet.totals();
    assert_eq!(totals.done, 4);
    assert_eq!(totals.failed, 0);
    // the fan-in task saw both deps
    assert_eq!(
        board.get_result(TaskId(3)).await.unwrap().output,
        "stub:3 deps=2"
    );
    let _ = TaskState::Done; // type is part of the public surface
}

/// End-to-end proof that independent tasks REALLY run at the same time —
/// through the public API, the real scheduler, and the same event stream the
/// broker and the chat pane consume.
///
/// A barrier is the proof, not a peak counter: each of the four independent
/// tasks blocks until all four have arrived, so the run can only finish if
/// they are genuinely in flight together. A serial scheduler cannot complete
/// this graph at all — it parks on the first task forever — so the outer
/// timeout failing IS the assertion. Counting `Running` events instead would
/// prove nothing here: stub agents finish instantly, so a serial run can
/// interleave to a peak of one and still look plausible.
///
/// Shape is the real one: a gather task, a fan-out of independent work, then
/// a fan-in — the pattern the LLM planner actually emits.
#[tokio::test]
async fn independent_tasks_really_run_at_the_same_time() {
    use crew_hive::board::TaskResult;
    use crew_hive::{Agent, AgentContext, AgentFactory};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Barrier;

    const WIDTH: usize = 4;

    struct Rendezvous {
        gate: Arc<Barrier>,
        peak: Arc<AtomicUsize>,
        live: Arc<AtomicUsize>,
    }
    impl Agent for Rendezvous {
        fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
            let (gate, peak, live) = (self.gate.clone(), self.peak.clone(), self.live.clone());
            Box::pin(async move {
                let n = live.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(n, Ordering::SeqCst);
                // The gather (0) and fan-in (5) tasks are alone in their wave,
                // so only the fan-out cohort rendezvouses — the others would
                // block forever waiting for siblings that cannot exist.
                if (1..=WIDTH as u64).contains(&ctx.task.id.0) {
                    gate.wait().await;
                }
                live.fetch_sub(1, Ordering::SeqCst);
                TaskResult {
                    task: ctx.task.id,
                    output: String::new(),
                    success: true,
                }
            })
        }
    }
    struct Factory {
        gate: Arc<Barrier>,
        peak: Arc<AtomicUsize>,
        live: Arc<AtomicUsize>,
    }
    impl AgentFactory for Factory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            Box::new(Rendezvous {
                gate: self.gate.clone(),
                peak: self.peak.clone(),
                live: self.live.clone(),
            })
        }
    }

    let g = TaskGraph::new(vec![
        spec(0, &[]),
        spec(1, &[0]),
        spec(2, &[0]),
        spec(3, &[0]),
        spec(4, &[0]),
        spec(5, &[1, 2, 3, 4]),
    ])
    .unwrap();

    let peak = Arc::new(AtomicUsize::new(0));
    let bus = EventBus::new(256);
    // Watch the same telemetry the chat pane's status line reads, so this also
    // pins that a parallel run is observable downstream and not just internal.
    let mut rx = bus.subscribe();
    let running = tokio::spawn(async move {
        let (mut live, mut peak) = (0usize, 0usize);
        while let Ok(ev) = rx.recv().await {
            if let crew_hive::HiveEvent::TaskStateChanged { state, .. } = &ev {
                match state {
                    TaskState::Done | TaskState::Failed | TaskState::Cancelled => {
                        live = live.saturating_sub(1)
                    }
                    TaskState::Running => {
                        live += 1;
                        peak = peak.max(live);
                    }
                    _ => {}
                }
            }
        }
        peak
    });

    let f = Arc::new(Factory {
        gate: Arc::new(Barrier::new(WIDTH)),
        peak: peak.clone(),
        live: Arc::new(AtomicUsize::new(0)),
    });
    let sched = Scheduler::new(g, Blackboard::new(), bus.clone(), f, 8).run();

    // A serial scheduler deadlocks on the barrier; fail loudly instead of
    // hanging the suite.
    let out = tokio::time::timeout(std::time::Duration::from_secs(10), sched)
        .await
        .expect(
            "timed out — the fan-out tasks never met at the barrier, so the \
             scheduler ran independent work serially",
        );
    drop(bus);

    assert_eq!(out.done.len(), 6, "every task completed");
    assert!(
        peak.load(Ordering::SeqCst) >= WIDTH,
        "peak in-flight was {}, expected {WIDTH} — the barrier released, so \
         they overlapped, but the count disagrees",
        peak.load(Ordering::SeqCst)
    );
    assert!(
        running.await.unwrap() >= WIDTH,
        "the event stream never showed {WIDTH} tasks Running at once, so a \
         parallel run is not observable downstream"
    );
}

#[tokio::test]
async fn plan_then_schedule_with_api_agents() {
    use crew_hive::{
        Agent, AgentFactory, AgentKind, ApiAgent, Blackboard, EventBus, MockProvider, Planner,
        Scheduler, StubPlanner,
    };
    use std::sync::Arc;

    struct ApiFactory {
        provider: Arc<MockProvider>,
    }
    impl AgentFactory for ApiFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            Box::new(ApiAgent::new(self.provider.clone(), 256))
        }
    }

    let graph = StubPlanner { fanout: 2 }
        .plan("build a thing")
        .await
        .unwrap();
    let n = graph.len();
    let board = Blackboard::new();
    let provider = Arc::new(MockProvider { reply: "ok".into() });
    let out = Scheduler::new(
        graph,
        board.clone(),
        EventBus::new(128),
        Arc::new(ApiFactory { provider }),
        8,
    )
    .run()
    .await;
    assert_eq!(out.done.len(), n);
    assert_eq!(board.result_count().await, n);
}

#[tokio::test]
async fn scheduler_runs_remote_agents() {
    use crew_hive::wire::{RemoteReply, RemoteTask};
    use crew_hive::worker::LoopbackTransport;
    use crew_hive::{Blackboard, EventBus, Planner, RemoteFactory, Scheduler, StubPlanner};
    use std::sync::Arc;

    // The exported RemoteFactory runs a whole graph over one shared transport;
    // an in-process LoopbackTransport stands in for a real sidecar worker.
    let transport = Arc::new(LoopbackTransport {
        handler: |t: RemoteTask| RemoteReply {
            task: t.task,
            output: "ok".into(),
            success: true,
            input_tokens: 1,
            output_tokens: 1,
        },
    });
    let factory = Arc::new(RemoteFactory::new(transport));

    let graph = StubPlanner { fanout: 3 }.plan("g").await.unwrap();
    let n = graph.len();
    let board = Blackboard::new();
    let out = Scheduler::new(graph, board.clone(), EventBus::new(128), factory, 8)
        .run()
        .await;
    assert_eq!(out.done.len(), n);
    assert_eq!(board.result_count().await, n);
}
