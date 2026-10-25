//! A scripted provider and a fake tool surface, for the tool-round tests.
use super::tests::spec;
use crate::agent::AgentContext;
use crate::bus::{AgentId, EventBus};
use crate::tools::Tools;
use std::sync::{Arc, Mutex};

/// A provider that answers with a scripted reply per call and RECORDS every
/// prompt it was given, so a test can assert what the agent actually saw on
/// the follow-up rather than merely that it succeeded.
pub(super) struct Scripted {
    replies: Mutex<Vec<String>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Scripted {
    pub(super) fn new(replies: &[&str]) -> (Arc<Self>, Arc<Mutex<Vec<String>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let p = Arc::new(Self {
            replies: Mutex::new(replies.iter().rev().map(|s| s.to_string()).collect()),
            seen: Arc::clone(&seen),
        });
        (p, seen)
    }
}

impl crate::provider::Provider for Scripted {
    fn complete(
        &self,
        req: crate::provider::CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<crate::provider::Completion, crate::provider::ProviderError>,
                > + Send,
        >,
    > {
        self.seen.lock().unwrap().push(req.prompt.clone());
        let text = self
            .replies
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| "out of script".to_string());
        Box::pin(async move {
            Ok(crate::provider::Completion {
                text,
                input_tokens: 1,
                output_tokens: 1,
                cost_microusd: 0,
                ..Default::default()
            })
        })
    }
}

/// A tool surface that records its calls and returns canned results.
pub(super) struct FakeTools {
    hint: String,
    calls: Arc<Mutex<Vec<String>>>,
    result: Result<String, String>,
}

impl Tools for FakeTools {
    fn hint(&self) -> String {
        self.hint.clone()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        self.result.clone()
    }
}

pub(super) fn fake(result: Result<String, String>) -> (Arc<FakeTools>, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    (
        Arc::new(FakeTools {
            hint: "TOOLS: @tool weather:current".into(),
            calls: Arc::clone(&calls),
            result,
        }),
        calls,
    )
}

pub(super) fn ctx(bus: &EventBus) -> AgentContext {
    AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(7),
        task: spec(1),
        deps: vec![],
        bus: bus.clone(),
    }
}
