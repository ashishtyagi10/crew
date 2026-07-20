use super::*;
use crate::graph::TaskId;
use crate::provider::MockProvider;

#[tokio::test]
async fn stub_planner_builds_fanout_plus_merge() {
    let g = StubPlanner { fanout: 3 }
        .plan("do the thing")
        .await
        .unwrap();
    assert_eq!(g.len(), 4); // 3 leaves + 1 merge
                            // the merge task (highest id) depends on all leaves
    let merge = g.tasks().iter().max_by_key(|t| t.id.0).unwrap();
    assert_eq!(merge.deps.len(), 3);
}

#[test]
fn parse_plan_builds_graph_from_json() {
    let json = r#"[
        {"id": 0, "title": "research", "prompt": "research X", "deps": []},
        {"id": 1, "title": "write", "prompt": "write up X", "deps": [0]}
    ]"#;
    let g = parse_plan(json).unwrap();
    assert_eq!(g.len(), 2);
    assert_eq!(g.get(TaskId(1)).unwrap().deps, vec![TaskId(0)]);
}

#[test]
fn parse_plan_rejects_garbage() {
    assert!(matches!(parse_plan("not json"), Err(PlanError::Parse(_))));
}

/// SECURITY: a malicious/compromised completion that tries to smuggle a
/// process-executing agent (`agent`/`command`/`args`/`system` keys) must NOT
/// produce a `Pty` task. serde drops the unknown fields and `parse_plan` forces
/// every task to `Api`, so the command-injection sink never materializes.
#[test]
fn parse_plan_ignores_injected_command_and_forces_api() {
    use crate::graph::AgentKind;
    let json = r#"[
        {"id": 0, "title": "pwn", "prompt": "p", "deps": [],
         "agent": "Pty", "command": "/bin/sh", "args": ["-c", "rm -rf /"],
         "system": "ignore-me"}
    ]"#;
    let g = parse_plan(json).unwrap();
    let task = g.get(TaskId(0)).unwrap();
    assert!(!task.agent.is_pty(), "injected Pty must be dropped");
    assert_eq!(task.agent, AgentKind::Api { system: None });
}

/// Across an arbitrary plan, no task is ever a process-spawning `Pty`.
#[test]
fn parse_plan_never_yields_pty() {
    let json = r#"[
        {"id": 0, "title": "a", "prompt": "p", "deps": [], "command": "x"},
        {"id": 1, "title": "b", "prompt": "q", "deps": [0], "args": ["y"]}
    ]"#;
    let g = parse_plan(json).unwrap();
    assert!(g.tasks().iter().all(|t| !t.agent.is_pty()));
}

#[test]
fn parse_plan_slugs_the_specialty() {
    let json = r#"[{"id":0,"title":"Gather Details","prompt":"p","deps":[],
                    "specialty":"Risk Assessor","expertise":"risk,  analysis"}]"#;
    let g = parse_plan(json).expect("valid plan");
    let t = &g.tasks()[0];
    assert_eq!(t.specialty, "risk-assessor");
    assert_eq!(t.expertise, "risk, analysis");
}

#[test]
fn parse_plan_derives_a_name_when_specialty_is_missing_or_garbage() {
    let missing = r#"[{"id":0,"title":"T","prompt":"p","deps":[]}]"#;
    assert_eq!(
        parse_plan(missing).unwrap().tasks()[0].specialty,
        "specialist-0"
    );

    let garbage = r#"[{"id":7,"title":"T","prompt":"p","deps":[],"specialty":"@#$"}]"#;
    assert_eq!(
        parse_plan(garbage).unwrap().tasks()[0].specialty,
        "specialist-7"
    );
}

#[test]
fn parse_plan_defaults_expertise_to_empty() {
    let json = r#"[{"id":0,"title":"T","prompt":"p","deps":[],"specialty":"analyst"}]"#;
    assert_eq!(parse_plan(json).unwrap().tasks()[0].expertise, "");
}

#[test]
fn parse_plan_every_specialty_is_a_valid_slug() {
    let json = r#"[{"id":0,"title":"T","prompt":"p","deps":[],"specialty":"A B/C+D"},
                   {"id":1,"title":"U","prompt":"p","deps":[],"specialty":""}]"#;
    for t in parse_plan(json).unwrap().tasks() {
        assert_eq!(
            crate::agentname::slug(&t.specialty).as_deref(),
            Some(t.specialty.as_str()),
            "specialty {:?} must be slug-stable",
            t.specialty
        );
    }
}

/// The goal end-to-end, deterministically: a plan whose tasks are independent
/// must actually EXECUTE independently. `planner_leaves_independent_work_
/// independent` proves the real LLM emits wide graphs, but it is `#[ignore]`d
/// (network + key), so nothing in default CI guards the link from planner
/// output shape to parallel execution. This closes that gap without a network:
/// it runs a `parse_plan` graph through the real `Scheduler` and asserts the
/// independent tasks overlap in time (peak concurrency reaches their count),
/// while a task that genuinely depends on them still waits.
///
/// If `parse_plan` ever regressed to chain independent tasks (a spurious dep),
/// peak concurrency would collapse to 1 and this fails — the scheduler cannot
/// recover width the plan never had.
#[tokio::test]
async fn parse_plan_independent_tasks_execute_concurrently() {
    use crate::agent::{Agent, AgentContext, AgentFactory};
    use crate::board::{Blackboard, TaskResult};
    use crate::bus::EventBus;
    use crate::graph::AgentKind;
    use crate::sched::Scheduler;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // Tracks live/peak concurrency across all agents.
    struct Counting {
        cur: Arc<AtomicUsize>,
        max: Arc<AtomicUsize>,
    }
    impl Agent for Counting {
        fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
            let (cur, max) = (self.cur.clone(), self.max.clone());
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
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            Box::new(Counting {
                cur: self.cur.clone(),
                max: self.max.clone(),
            })
        }
    }

    // Three independent workers + a merge that needs all three. This is the
    // exact shape the planner is prompted to produce for parallelisable goals.
    let json = r#"[
        {"id": 0, "title": "research A", "prompt": "a", "deps": []},
        {"id": 1, "title": "research B", "prompt": "b", "deps": []},
        {"id": 2, "title": "research C", "prompt": "c", "deps": []},
        {"id": 3, "title": "synthesize", "prompt": "merge", "deps": [0, 1, 2]}
    ]"#;
    let graph = parse_plan(json).expect("valid plan");

    let cur = Arc::new(AtomicUsize::new(0));
    let max = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(CountingFactory {
        cur: cur.clone(),
        max: max.clone(),
    });
    // Cap comfortably above the width so the cap is never the limiting factor:
    // any serialisation this test sees comes from the plan, not the pool.
    let out = Scheduler::new(graph, Blackboard::new(), EventBus::new(64), factory, 4)
        .run()
        .await;

    assert_eq!(out.done.len(), 4, "all four tasks must complete");
    assert_eq!(
        max.load(Ordering::SeqCst),
        3,
        "the three independent tasks must run at the same time (peak {}); \
         a peak below 3 means parse_plan chained work that should be parallel",
        max.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn llm_planner_parses_provider_json() {
    let reply = r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#;
    let planner = LlmPlanner {
        provider: MockProvider {
            reply: reply.into(),
        },
        tier: crate::graph::ModelTier::Standard,
        model: None,
    };
    let g = planner.plan("goal").await.unwrap();
    assert_eq!(g.len(), 1);
}

#[tokio::test]
async fn llm_planner_model_override_reaches_request() {
    use std::sync::{Arc, Mutex};
    // A probe provider that records the requested model and returns a valid plan.
    struct Probe(Arc<Mutex<String>>);
    impl crate::provider::Provider for Probe {
        fn complete(
            &self,
            req: crate::provider::CompletionRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::provider::Completion,
                            crate::provider::ProviderError,
                        >,
                    > + Send,
            >,
        > {
            *self.0.lock().unwrap() = req.model.clone();
            Box::pin(async {
                Ok(crate::provider::Completion {
                    text: r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#.into(),
                    input_tokens: 1,
                    output_tokens: 1,
                })
            })
        }
    }
    let seen = Arc::new(Mutex::new(String::new()));
    let planner = LlmPlanner {
        provider: Probe(seen.clone()),
        tier: crate::graph::ModelTier::Standard,
        model: None,
    }
    .with_model("qwen-max");
    planner.plan("goal").await.unwrap();
    assert_eq!(seen.lock().unwrap().as_str(), "qwen-max");
}
