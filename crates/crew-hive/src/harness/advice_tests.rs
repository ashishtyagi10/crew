//! The two things an engine hands a turn beyond its tools: an [`Advice`] read
//! each round, and a prompt with the `@tool` hint already placed.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::*;
use crate::provider::{Completion, CompletionRequest, ProviderError};

/// Asks for `fs:read` once, then answers; keeps every prompt.
struct Once(Mutex<u32>, Arc<Mutex<Vec<String>>>);

impl Provider for Once {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.1.lock().unwrap().push(req.prompt);
        let mut n = self.0.lock().unwrap();
        *n += 1;
        let text = match *n {
            1 => "@tool fs:read {}".to_string(),
            _ => "done".to_string(),
        };
        Box::pin(async move {
            Ok(Completion {
                text,
                ..Default::default()
            })
        })
    }
}

struct Read;

impl Tools for Read {
    fn hint(&self) -> String {
        "TOOLS: fs:read".into()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        Ok("FILE".into())
    }
}

/// Says "ADVICE" about every round, and counts the rounds it read.
struct Says(Mutex<Vec<String>>);

impl Advice for Says {
    fn read(&self, calls: &[ToolCall], ran: &[(bool, String)]) -> String {
        self.0
            .lock()
            .unwrap()
            .push(format!("{} {}", calls[0].label(), ran[0].1));
        "ADVICE\n\n".into()
    }
}

async fn turn(hinted: Option<String>, stop: bool) -> (Vec<String>, Arc<Says>) {
    let prompts: Arc<Mutex<Vec<String>>> = Arc::default();
    let says = Arc::new(Says(Mutex::default()));
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::new(EventBus::new(64)),
        budget: ToolBudget::turn(4),
        cancel: Arc::new(AtomicBool::new(false)),
        advice: Some(Arc::clone(&says) as Arc<dyn Advice>),
    };
    if stop {
        // A stop as the read's result lands: the advice is not asked for.
        struct Stopping(Arc<AtomicBool>);
        impl Tools for Stopping {
            fn hint(&self) -> String {
                Read.hint()
            }
            fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
                self.0.store(true, Ordering::Relaxed);
                Ok("FILE".into())
            }
        }
        let tools: Arc<dyn Tools> = Arc::new(Stopping(Arc::clone(&ctx.cancel)));
        let spec = spec(hinted);
        let model = Arc::new(Once(Mutex::default(), Arc::clone(&prompts)));
        run(ctx, spec, model, Some(tools), None).await;
    } else {
        let spec = spec(hinted);
        let model = Arc::new(Once(Mutex::default(), Arc::clone(&prompts)));
        run(ctx, spec, model, Some(Arc::new(Read)), None).await;
    }
    let prompts = prompts.lock().unwrap().clone();
    (prompts, says)
}

fn spec(hinted: Option<String>) -> TurnSpec {
    TurnSpec {
        ask: "read it".into(),
        prompt: "TASK: read it\n\nHOW TO REPLY: end with @done".into(),
        hinted,
        system: None,
        model: "m".into(),
        tier: ModelTier::Standard,
        max_tokens: 64,
    }
}

#[tokio::test]
async fn advice_is_read_each_round_and_rides_the_follow_up() {
    let (prompts, says) = turn(None, false).await;
    assert_eq!(*says.0.lock().unwrap(), ["fs:read FILE"]);
    assert_eq!(prompts.len(), 2);
    let follow = &prompts[1];
    let (exchanges, budget) = (
        follow.find("RESULT:").unwrap(),
        follow.find("You may").unwrap(),
    );
    let advice = follow.find("ADVICE").unwrap();
    assert!(exchanges < advice && advice < budget, "{follow}");
}

#[tokio::test]
async fn a_stopped_turn_is_not_advised() {
    let (_, says) = turn(None, true).await;
    assert!(says.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_text_loop_takes_the_hint_where_the_engine_put_it() {
    let placed = "TASK: read it\nTOOLS: fs:read\n\nHOW TO REPLY: end with @done".to_string();
    let (prompts, _) = turn(Some(placed.clone()), false).await;
    assert_eq!(prompts[0], placed);
    assert!(prompts[1].starts_with(&placed), "{}", prompts[1]);
    assert_eq!(
        prompts[1].matches("TOOLS: fs:read").count(),
        1,
        "{}",
        prompts[1]
    );
    let (prompts, _) = turn(None, false).await;
    assert!(prompts[0].ends_with("TOOLS: fs:read"), "{}", prompts[0]);
}
