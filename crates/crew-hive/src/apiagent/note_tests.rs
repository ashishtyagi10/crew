use super::*;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crate::tools::{ToolSpec, Tools};
use std::sync::{Arc, Mutex};

#[test]
fn no_note_leaves_system_and_prompt_byte_identical() {
    let (s, p) = noted(Some("be brief".into()), "weather?".into(), None);
    assert_eq!(s.as_deref(), Some("be brief"));
    assert_eq!(p, "weather?");
    let (s, p) = noted(None, "weather?".into(), None);
    assert_eq!(s, None);
    assert_eq!(p, "weather?");
}

#[test]
fn the_note_joins_the_system_prompt_when_there_is_one() {
    let (s, p) = noted(
        Some("be brief".into()),
        "weather?".into(),
        Some("3 more".into()),
    );
    assert_eq!(s.as_deref(), Some("be brief\n\n3 more"));
    assert_eq!(
        p, "weather?",
        "the user turn is untouched when a system carries the note"
    );
}

#[test]
fn the_note_joins_the_user_turn_when_there_is_no_system() {
    let (s, p) = noted(None, "weather?".into(), Some("3 more".into()));
    assert_eq!(s, None, "no system prompt is invented to hold a footnote");
    assert_eq!(p, "weather?\n\n3 more");
}

/// A tool-speaking provider that answers once and keeps the request it saw.
struct Seeing(Arc<Mutex<Option<CompletionRequest>>>);

impl Provider for Seeing {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        *self.0.lock().unwrap() = Some(req);
        Box::pin(async {
            Ok(Completion {
                text: "sunny".into(),
                ..Default::default()
            })
        })
    }
}

/// One tool on the wire, and a note only when the test says so.
struct Partial(Option<String>);

impl Tools for Partial {
    fn hint(&self) -> String {
        "unused on the native path".into()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        Ok(String::new())
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "weather".into(),
            tool: "current".into(),
            description: "current conditions".into(),
            input_schema: serde_json::json!({"type":"object"}),
        }]
    }
    fn note_for(&self, _task: &str) -> Option<String> {
        self.0.clone()
    }
}

/// Run the full agent on the native branch and hand back the request it sent.
async fn request_seen(system: Option<&str>, note: Option<&str>) -> CompletionRequest {
    let seen = Arc::new(Mutex::new(None));
    let bus = EventBus::new(64);
    let ctx = AgentContext {
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(3),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api {
                system: system.map(str::to_string),
            },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "weather?".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    crate::apiagent::ApiAgent::new(Arc::new(Seeing(Arc::clone(&seen))), 256)
        .with_tools(Arc::new(Partial(note.map(str::to_string))))
        .run(ctx)
        .await;
    let req = seen.lock().unwrap().take();
    req.expect("the provider was called")
}

#[tokio::test]
async fn a_native_run_with_nothing_omitted_sends_the_prompt_it_always_did() {
    let req = request_seen(Some("be brief"), None).await;
    assert_eq!(req.system.as_deref(), Some("be brief"));
    assert_eq!(req.prompt, "weather?");
    assert_eq!(req.tools.len(), 1, "the tools are still on the wire");
}

#[tokio::test]
async fn a_native_run_tells_the_model_in_the_system_prompt_what_was_left_off_the_wire() {
    let note = "16 more tool(s) are connected; sys__find_tools searches them";
    let req = request_seen(Some("be brief"), Some(note)).await;
    assert_eq!(
        req.system.as_deref(),
        Some(format!("be brief\n\n{note}").as_str())
    );
    assert_eq!(req.prompt, "weather?");
}

#[tokio::test]
async fn a_native_run_without_a_system_prompt_puts_the_note_on_the_user_turn() {
    let req = request_seen(None, Some("16 more")).await;
    assert_eq!(req.system, None);
    assert_eq!(req.prompt, "weather?\n\n16 more");
}
