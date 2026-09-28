//! [`Tools::failed`] is asked wherever a swarm turns a result into ok: the
//! `@tool` text path and the bridge a sidecar calls through. The native path
//! is held by crew-plugin's `runflag_tests`, against a real shell.
use std::sync::Arc;

use super::Tools;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};

const RAN_AND_FAILED: &str = "exit 3\nx\n";

/// A shell whose command fails, on a surface that knows how to say so.
struct Exits;

impl Tools for Exits {
    fn hint(&self) -> String {
        "TOOLS: @tool sys:run {\"cmd\": …}".into()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        Ok(RAN_AND_FAILED.into())
    }
    fn failed(&self, _s: &str, _t: &str, output: &str) -> bool {
        output.starts_with("exit 3")
    }
}

/// The same result on a surface that never heard of `failed`.
struct Silent;

impl Tools for Silent {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        Ok(RAN_AND_FAILED.into())
    }
}

#[test]
fn the_bridge_tells_a_sidecar_the_call_failed() {
    let on_delta = |_: &str| {};
    let host = crate::wire::Host {
        tools: Some(&Exits),
        on_delta: &on_delta,
    };
    assert_eq!(host.call("sys:run", "{}"), (RAN_AND_FAILED.into(), false));
    let host = crate::wire::Host {
        tools: Some(&Silent),
        on_delta: &on_delta,
    };
    assert!(host.call("sys:run", "{}").1, "by default an Ok is ok");
}

/// A model on the text convention: asks once, then answers.
struct Asks;

impl Provider for Asks {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let text = match req.prompt.contains("TOOL EXCHANGES SO FAR") {
            true => "it failed",
            false => "@tool sys:run {}",
        };
        Box::pin(async move {
            Ok(Completion {
                text: text.into(),
                input_tokens: 1,
                output_tokens: 1,
                ..Default::default()
            })
        })
    }
}

#[tokio::test]
async fn the_text_path_publishes_a_failed_line() {
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: super::budget::ToolBudget::solo(),
        agent: AgentId(1),
        task: TaskSpec {
            id: TaskId(1),
            title: "build".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "run the build".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    crate::apiagent::ApiAgent::new(Arc::new(Asks), 256)
        .with_tools(Arc::new(Exits))
        .run(ctx)
        .await;
    let oks: Vec<bool> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|e| match e {
            HiveEvent::ToolResult { ok, .. } => Some(ok),
            _ => None,
        })
        .collect();
    assert_eq!(oks, vec![false]);
}
