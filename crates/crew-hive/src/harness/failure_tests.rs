//! A failed task hands back WHY: the planner re-planning after it reads the
//! provider's error after the colon, and the pane's `Failed` event carries
//! the same words.
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use super::*;
use crate::apiagent::ApiFactory;
use crate::board::Blackboard;
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::planner::{PlanError, Planner};
use crate::provider::{Completion, CompletionRequest, Provider};
use crate::sched::Scheduler;

/// Every call fails with the host saying it has no such model.
struct NoSuchModel;

impl Provider for NoSuchModel {
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        Box::pin(async { Err(ProviderError::Api("model qwen-maxx does not exist".into())) })
    }
}

/// Records the re-plan prompt and offers no replacement.
struct Recorder(Arc<Mutex<Vec<String>>>);

impl Planner for Recorder {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        self.0.lock().unwrap().push(goal.to_string());
        Box::pin(async { Err(PlanError::Parse("no replacement".into())) })
    }
}

#[tokio::test]
async fn the_replan_prompt_reads_the_provider_error_after_the_colon() {
    let task = TaskSpec {
        id: TaskId(1),
        title: "t1".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: "look it up".into(),
        specialty: String::new(),
        expertise: String::new(),
    };
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    let out = Scheduler::new(
        TaskGraph::new(vec![task]).unwrap(),
        Blackboard::new(),
        bus,
        Arc::new(ApiFactory::new(Arc::new(NoSuchModel), 64)),
        1,
    )
    .with_replan("the goal", Arc::new(Recorder(Arc::clone(&prompts))))
    .run()
    .await;
    assert_eq!(out.failed, vec![TaskId(1)], "{out:?}");
    let prompts = prompts.lock().unwrap();
    assert_eq!(prompts.len(), 1, "one re-plan");
    let said = "FAILED task \u{201c}t1\u{201d}: api error: model qwen-maxx does not exist";
    assert!(prompts[0].contains(said), "{}", prompts[0]);
    // The pane hears the very same words.
    let mut shown = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let crate::bus::HiveEvent::Failed { error, .. } = ev {
            shown.push(error);
        }
    }
    assert_eq!(shown, vec!["api error: model qwen-maxx does not exist"]);
}

#[test]
fn a_short_error_is_said_whole() {
    assert_eq!(
        reason(&"  http error: x went quiet \n"),
        "http error: x went quiet"
    );
}

#[test]
fn a_long_error_keeps_whole_lines() {
    let line = "x".repeat(120);
    let said = format!("{line}\n{line}\n{line}");
    let got = reason(&said);
    assert_eq!(got, format!("{line}\n{line}"), "cut at the line that fits");
}

#[test]
fn one_overlong_line_is_cut_and_says_so() {
    let got = reason(&"y".repeat(1000));
    assert_eq!(got.chars().count(), REASON_CAP + 1);
    assert!(got.ends_with('\u{2026}'), "{got}");
}
