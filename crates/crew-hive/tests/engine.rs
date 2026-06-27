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

#[tokio::test]
async fn plan_then_schedule_with_api_agents() {
    use crew_hive::{
        Agent, AgentFactory, AgentKind, ApiAgent, Blackboard, EventBus, MockProvider, ModelTier,
        Planner, Scheduler, StubPlanner,
    };
    use std::sync::Arc;

    struct ApiFactory {
        provider: Arc<MockProvider>,
    }
    impl AgentFactory for ApiFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            Box::new(ApiAgent::new(
                self.provider.clone(),
                ModelTier::Standard,
                256,
            ))
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
    use crew_hive::{
        Agent, AgentFactory, AgentKind, Blackboard, EventBus, Planner, RemoteAgent, Scheduler,
        StubPlanner,
    };
    use std::sync::Arc;

    struct RemoteFactory;
    impl AgentFactory for RemoteFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            let tr = LoopbackTransport {
                handler: |t: RemoteTask| RemoteReply {
                    task: t.task,
                    output: "ok".into(),
                    success: true,
                    input_tokens: 1,
                    output_tokens: 1,
                },
            };
            Box::new(RemoteAgent::new(Arc::new(tr)))
        }
    }

    let graph = StubPlanner { fanout: 3 }.plan("g").await.unwrap();
    let n = graph.len();
    let board = Blackboard::new();
    let out = Scheduler::new(
        graph,
        board.clone(),
        EventBus::new(128),
        Arc::new(RemoteFactory),
        8,
    )
    .run()
    .await;
    assert_eq!(out.done.len(), n);
    assert_eq!(board.result_count().await, n);
}
