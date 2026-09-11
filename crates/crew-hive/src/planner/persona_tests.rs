use super::*;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::planner::{parse_plan, PlanError, Planner};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[test]
fn a_named_specialty_gets_a_persona_naming_it_and_its_expertise() {
    let json = r#"[{"id":0,"title":"Review the diff","prompt":"p","deps":[],
                    "specialty":"code-reviewer","expertise":"diffs, correctness, style"}]"#;
    let g = parse_plan(json).unwrap();
    let AgentKind::Api { system: Some(s) } = &g.tasks()[0].agent else {
        panic!(
            "a named specialty must carry a system prompt: {:?}",
            g.tasks()[0].agent
        );
    };
    assert!(s.contains("You are the code-reviewer."), "{s}");
    assert!(s.contains("diffs, correctness, style"), "{s}");
    assert!(
        s.contains("Review the diff"),
        "the task's title places it: {s}"
    );
    assert!(s.contains("one worker in a larger plan"), "{s}");
    assert!(s.contains("deliverable"), "{s}");
}

#[test]
fn an_empty_or_garbage_specialty_keeps_the_system_prompt_absent() {
    let json = r#"[{"id":0,"title":"T","prompt":"p","deps":[]},
                   {"id":1,"title":"U","prompt":"p","deps":[],"specialty":""},
                   {"id":2,"title":"V","prompt":"p","deps":[],"specialty":"@#$"}]"#;
    for t in parse_plan(json).unwrap().tasks() {
        assert_eq!(t.agent, AgentKind::Api { system: None }, "task {}", t.id.0);
        assert!(t.specialty.starts_with("specialist-"), "{}", t.specialty);
    }
}

#[test]
fn the_persona_does_not_read_specialty_is_dot_when_expertise_is_empty() {
    let s = worker("archivist", "", "Gather details");
    assert!(
        s.starts_with("You are the archivist. You are one worker"),
        "{s}"
    );
    assert!(!s.contains("specialty is ."), "{s}");
    assert_eq!(identity("archivist", ""), "You are the archivist.");
    assert_eq!(
        identity("archivist", "records"),
        "You are the archivist. Your specialty is records."
    );
}

/// Records the agent kind every run actually executed under.
struct Recording(Arc<Mutex<Vec<(TaskId, AgentKind)>>>);

impl crate::agent::AgentFactory for Recording {
    fn make(&self, _k: &AgentKind) -> Box<dyn crate::agent::Agent> {
        struct A(Arc<Mutex<Vec<(TaskId, AgentKind)>>>);
        impl crate::agent::Agent for A {
            fn run(
                &self,
                ctx: crate::agent::AgentContext,
            ) -> Pin<Box<dyn Future<Output = crate::board::TaskResult> + Send>> {
                let id = ctx.task.id;
                self.0.lock().unwrap().push((id, ctx.task.agent.clone()));
                // Task 0 fails so the scheduler asks for a re-plan.
                let ok = id != TaskId(0);
                Box::pin(async move {
                    crate::board::TaskResult {
                        task: id,
                        output: format!("out:{}", id.0),
                        success: ok,
                    }
                })
            }
        }
        Box::new(A(Arc::clone(&self.0)))
    }
}

/// A planner that answers, as a real provider would, with JSON that goes
/// through `parse_plan` — so the replacement carries whatever it mints.
struct Json(&'static str);

impl Planner for Json {
    fn plan(
        &self,
        _goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        let out = parse_plan(self.0);
        Box::pin(async move { out })
    }
}

#[tokio::test]
async fn a_replanned_replacement_task_carries_its_persona_too() {
    let first = TaskSpec {
        id: TaskId(0),
        title: "first".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: String::new(),
        specialty: "starter".into(),
        expertise: String::new(),
    };
    let seen = Arc::new(Mutex::new(Vec::new()));
    let planner = Arc::new(Json(
        r#"[{"id":0,"title":"Retry it","prompt":"p","deps":[],
             "specialty":"fixer","expertise":"repairs"}]"#,
    ));
    let out = crate::sched::Scheduler::new(
        TaskGraph::new(vec![first]).unwrap(),
        crate::board::Blackboard::new(),
        crate::bus::EventBus::new(64),
        Arc::new(Recording(Arc::clone(&seen))),
        1,
    )
    .with_replan("ship it", planner)
    .run()
    .await;
    // Replacement ids are remapped past the old graph's maximum (0 → 1).
    assert!(
        out.done.contains(&TaskId(1)),
        "the replacement ran: {out:?}"
    );
    let seen = seen.lock().unwrap();
    let (_, kind) = seen
        .iter()
        .find(|(id, _)| *id == TaskId(1))
        .expect("the replacement task was executed");
    let AgentKind::Api { system: Some(s) } = kind else {
        panic!("the replacement must keep its persona: {kind:?}");
    };
    assert!(
        s.contains("You are the fixer. Your specialty is repairs."),
        "{s}"
    );
    assert!(s.contains("Retry it"), "{s}");
}
