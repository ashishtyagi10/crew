use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::plan_with_repair;
use crate::planner::PlanError;
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};

const PLAN: &str = r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#;

/// Answers from a queue, counting calls and keeping every prompt it was sent.
struct Scripted {
    replies: Mutex<VecDeque<Result<&'static str, &'static str>>>,
    calls: AtomicUsize,
    prompts: Mutex<Vec<String>>,
}

impl Scripted {
    fn new(replies: &[Result<&'static str, &'static str>]) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.iter().copied().collect()),
            calls: AtomicUsize::new(0),
            prompts: Mutex::new(Vec::new()),
        })
    }
}

impl Provider for Scripted {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.prompts.lock().unwrap().push(req.prompt);
        let next = self.replies.lock().unwrap().pop_front();
        Box::pin(async move {
            match next.expect("the script ran out of replies") {
                Ok(text) => Ok(Completion {
                    text: text.into(),
                    ..Default::default()
                }),
                Err(e) => Err(ProviderError::Api(e.into())),
            }
        })
    }
}

fn req() -> CompletionRequest {
    CompletionRequest {
        model: "m".into(),
        system: Some("plan".into()),
        prompt: "the goal".into(),
        max_tokens: 64,
        ..Default::default()
    }
}

#[tokio::test]
async fn a_fenced_reply_with_a_sentence_before_it_plans_without_a_re_ask() {
    let reply = "Sure, here is the plan:\n```json\n[{\"id\":0,\"title\":\"t\",\"prompt\":\"p\",\"deps\":[],},]\n```";
    let p = Scripted::new(&[Ok(reply)]);
    let graph = plan_with_repair(Arc::clone(&p), req()).await.unwrap();
    assert_eq!(graph.len(), 1);
    assert_eq!(p.calls.load(Ordering::SeqCst), 1, "no re-ask was needed");
}

#[tokio::test]
async fn a_reply_that_fails_once_is_re_asked_with_the_error_and_the_second_plans() {
    let p = Scripted::new(&[Ok("I would rather not."), Ok(PLAN)]);
    let graph = plan_with_repair(Arc::clone(&p), req()).await.unwrap();
    assert_eq!(graph.len(), 1);
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let prompts = p.prompts.lock().unwrap();
    assert_eq!(prompts[0], "the goal");
    assert!(prompts[1].starts_with("the goal\n"), "{}", prompts[1]);
    assert!(prompts[1].contains("no JSON array"), "{}", prompts[1]);
    assert!(prompts[1].contains("I would rather not."), "{}", prompts[1]);
    assert!(prompts[1].contains("ONLY the JSON array"), "{}", prompts[1]);
}

#[tokio::test]
async fn a_reply_that_fails_twice_is_a_plan_error_after_exactly_two_calls() {
    let p = Scripted::new(&[Ok("nope"), Ok("[{\"id\":\"zero\"}]"), Ok(PLAN)]);
    let err = plan_with_repair(Arc::clone(&p), req()).await.unwrap_err();
    assert_eq!(p.calls.load(Ordering::SeqCst), 2, "one re-ask, never two");
    match err {
        PlanError::Parse(s) => assert!(s.ends_with("after one repair re-ask"), "{s}"),
        other => panic!("expected a parse error, got {other}"),
    }
}

#[tokio::test]
async fn a_provider_error_is_surfaced_at_once_and_never_re_asked() {
    let p = Scripted::new(&[Err("overloaded"), Ok(PLAN)]);
    let err = plan_with_repair(Arc::clone(&p), req()).await.unwrap_err();
    assert!(matches!(err, PlanError::Provider(_)), "{err}");
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn the_repair_prompt_bounds_the_reply_it_shows_back() {
    let long = "x".repeat(10_000);
    let prompt = super::repair_prompt("g", &long, &PlanError::Parse("e".into()));
    assert!(prompt.len() < 5_000, "{}", prompt.len());
}
