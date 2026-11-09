//! What the stop-check tests share: a provider that numbers its answers (or
//! repeats one), a surface whose check holds a set number of times, and one
//! whole turn run on them with the `hook:stop` rows it showed.
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use super::super::{run, Ended, Events, TurnContext, TurnSpec};
use super::LABEL;
use crate::bus::{AgentId, HiveEvent};
use crate::graph::ModelTier;
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crate::tools::{budget::ToolBudget, StopCheck, ToolSpec, Tools};

/// Answers `answer` plus the call's number, or the same `answer` every time.
struct Answers {
    answer: &'static str,
    numbered: bool,
    /// Tool-use on the wire, or the `@tool` text loop.
    native: bool,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl Provider for Answers {
    fn supports_tools(&self) -> bool {
        self.native
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let mut seen = self.seen.lock().unwrap();
        seen.push(req);
        let text = match self.numbered {
            true => format!("{} {}", self.answer, seen.len()),
            false => self.answer.to_string(),
        };
        Box::pin(async move {
            Ok(Completion {
                text,
                ..Default::default()
            })
        })
    }
}

/// One tool on the wire (so the native loop runs) and a stop check that
/// holds the first `holds` answers, or breaks when `holds` is `None`.
struct Checked {
    holds: Option<u32>,
    asked: AtomicU32,
    pub(super) flags: Mutex<Vec<bool>>,
}

impl Tools for Checked {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        Ok(String::new())
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "sys".into(),
            tool: "read".into(),
            description: "reads".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }]
    }
    fn stop(&self, _answer: &str, held: bool) -> StopCheck {
        self.flags.lock().unwrap().push(held);
        let n = self.asked.fetch_add(1, Ordering::Relaxed);
        match self.holds {
            None => StopCheck::Broke("ran past its 5s deadline and was stopped".into()),
            Some(h) if n < h => StopCheck::Hold("the tests fail".into()),
            Some(_) => StopCheck::End,
        }
    }
}

#[derive(Default)]
struct Kept(Mutex<Vec<HiveEvent>>);

impl Events for Kept {
    fn publish(&self, ev: HiveEvent) {
        self.0.lock().unwrap().push(ev);
    }
}

pub(super) struct Ran {
    pub(super) ended: Ended,
    pub(super) requests: Vec<CompletionRequest>,
    /// The `hook:stop` rows: ok, and the words.
    pub(super) rows: Vec<(bool, String)>,
    pub(super) flags: Vec<bool>,
}

pub(super) async fn turn(holds: Option<u32>, numbered: bool) -> Ran {
    turn_on(holds, numbered, true).await
}

pub(super) async fn turn_on(holds: Option<u32>, numbered: bool, native: bool) -> Ran {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(Answers {
        answer: "done",
        numbered,
        native,
        seen: Arc::clone(&seen),
    });
    let tools = Arc::new(Checked {
        holds,
        asked: AtomicU32::new(0),
        flags: Mutex::default(),
    });
    let events = Arc::new(Kept::default());
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::clone(&events) as Arc<dyn Events>,
        budget: ToolBudget::turn(4),
        cancel: Default::default(),
        advice: None,
    };
    let spec = TurnSpec {
        ask: "fix it".into(),
        prompt: "fix it".into(),
        hinted: None,
        shed: None,
        ending: None,
        call_timeout: None,
        system: None,
        model: "m".into(),
        tier: ModelTier::Standard,
        max_tokens: 64,
    };
    let ended = run(ctx, spec, provider, Some(tools.clone()), None).await;
    let rows = events
        .0
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .filter_map(|e| match e {
            HiveEvent::ToolResult {
                label, ok, text, ..
            } if label == LABEL => Some((ok, text)),
            _ => None,
        });
    let rows = rows.collect();
    let requests = seen.lock().unwrap().clone();
    let flags = tools.flags.lock().unwrap().clone();
    Ran {
        ended,
        requests,
        rows,
        flags,
    }
}
