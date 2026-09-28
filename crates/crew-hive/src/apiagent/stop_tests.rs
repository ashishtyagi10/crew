//! A stop pressed while a worker's tool runs: no model call after it, and no
//! tool after it either. The tool itself presses stop, as a person would
//! while a build ran, so the moment is exact rather than timed.
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::agent::{Agent, AgentContext, Attempt};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError, ToolInvocation};
use crate::tools::ToolSpec;

/// Asks for tools until its script runs out, then answers; counts every call
/// as it is MADE (a real provider's request goes out then, not when polled).
struct Model {
    native: bool,
    script: Mutex<Vec<Completion>>,
    calls: Arc<AtomicUsize>,
    /// How long each call takes to answer.
    wait: Duration,
}

impl Provider for Model {
    fn supports_tools(&self) -> bool {
        self.native
    }
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let next = self.script.lock().unwrap().pop();
        let wait = self.wait;
        Box::pin(async move {
            tokio::time::sleep(wait).await;
            Ok(next.unwrap_or_else(|| Completion {
                text: "the answer".into(),
                ..Default::default()
            }))
        })
    }
}

/// A tool whose every call presses stop, and which counts its runs.
struct Stopping {
    cancel: Arc<AtomicBool>,
    ran: Arc<AtomicUsize>,
}

impl Tools for Stopping {
    fn hint(&self) -> String {
        "TOOLS: fs:build".into()
    }
    fn call(&self, _server: &str, _tool: &str, _args: &str) -> Result<String, String> {
        self.ran.fetch_add(1, Ordering::SeqCst);
        self.cancel.store(true, Ordering::Relaxed);
        Ok("built".into())
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "fs".into(),
            tool: "build".into(),
            description: "build it".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }]
    }
}

/// Run one worker whose first reply is `first`: (attempt, model calls, tool runs).
async fn run(native: bool, first: Completion) -> (Attempt, usize, usize) {
    let cancel = Arc::new(AtomicBool::new(false));
    run_with(native, first, Duration::ZERO, cancel).await
}

/// [`run`], each model call taking `wait`, stopped through `cancel`.
async fn run_with(
    native: bool,
    first: Completion,
    wait: Duration,
    cancel: Arc<AtomicBool>,
) -> (Attempt, usize, usize) {
    let (calls, ran) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let model = Model {
        native,
        script: Mutex::new(vec![first]),
        calls: Arc::clone(&calls),
        wait,
    };
    let tools = Stopping {
        cancel: Arc::clone(&cancel),
        ran: Arc::clone(&ran),
    };
    let agent = ApiAgent::new(Arc::new(model), 64).with_tools(Arc::new(tools));
    let ctx = AgentContext {
        cancel,
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(0),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "build it".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: EventBus::new(64),
    };
    let attempt = agent.attempt(ctx).await;
    (
        attempt,
        calls.load(Ordering::SeqCst),
        ran.load(Ordering::SeqCst),
    )
}

fn text(reply: &str) -> Completion {
    Completion {
        text: reply.into(),
        ..Default::default()
    }
}

fn assert_stopped(a: &Attempt) {
    assert!(a.stopped, "not stopped: {:?}", a.result.output);
    assert!(!a.result.success && !a.transient, "{:?}", a.result.output);
}

#[tokio::test]
async fn a_text_worker_makes_no_model_call_after_a_stop_during_its_tool() {
    let (attempt, calls, ran) = run(false, text("building\n@tool fs:build {}")).await;
    assert_eq!(ran, 1, "the tool ran");
    assert_eq!(calls, 1, "a model call went out after the stop");
    assert_stopped(&attempt);
}

/// A native reply asking for the build.
fn asks() -> Completion {
    Completion {
        calls: vec![ToolInvocation {
            id: "c1".into(),
            name: "fs__build".into(),
            input: serde_json::json!({}),
            bad_args: None,
        }],
        ..Default::default()
    }
}

#[tokio::test]
async fn a_native_worker_makes_no_model_call_after_a_stop_during_its_tool() {
    let (attempt, calls, ran) = run(true, asks()).await;
    assert_eq!(ran, 1, "the tool ran");
    assert_eq!(calls, 1, "a model call went out after the stop");
    assert_stopped(&attempt);
}

/// Two calls in one reply run in turn; a stop during the first keeps the
/// second from starting, and it is answered as not run.
#[tokio::test]
async fn a_stop_during_one_tool_keeps_the_next_from_starting() {
    let (attempt, calls, ran) = run(false, text("@tool fs:build {}\n@tool fs:build {}")).await;
    assert_eq!(ran, 1, "the second tool started after the stop");
    assert_eq!(calls, 1);
    assert_stopped(&attempt);
}

/// The native loop's call in flight is dropped at the stop, as the text
/// loop's is (`sched::stop_tests` drives that one through the scheduler).
#[tokio::test]
async fn a_native_worker_drops_its_call_in_flight_at_the_stop() {
    let cancel = Arc::new(AtomicBool::new(false));
    let setter = Arc::clone(&cancel);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        setter.store(true, Ordering::Relaxed);
    });
    let started = Instant::now();
    let (attempt, calls, ran) = run_with(true, asks(), Duration::from_secs(5), cancel).await;
    let took = started.elapsed();
    assert!(
        took < Duration::from_millis(1500),
        "waited {took:?} on a 5 s call"
    );
    assert_eq!((calls, ran), (1, 0));
    assert_stopped(&attempt);
}
