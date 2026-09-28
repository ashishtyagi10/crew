//! What the overflow tests share: a model's refusals as the hosts word them,
//! a file surface, a task, and a run of the whole agent that counts the
//! notes the pane was sent.
use crate::agent::{Agent, AgentContext};
use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::Provider;
use crate::tools::exchanges::CONTEXT_FULL;
use crate::tools::{ToolSpec, Tools};
use std::sync::{Arc, Mutex};

pub(super) const DASHSCOPE: &str = r#"{"error":{"message":"Range of input length should be [1, 30720]","code":"invalid_parameter_error"}}"#;
pub(super) const BAD_INPUT: &str = r#"{"error":{"message":"<400> InternalError.Algo.InvalidParameter: An unknown error occurred due to an unsupported input format.","code":"invalid_parameter_error"}}"#;

/// Whether a model with a window of `limit` chars refuses a request of
/// `chars`, the `sent`-th (from 0); from the `from`-th on it refuses them all.
pub(super) fn refuses(sent: usize, chars: usize, limit: usize, from: Option<usize>) -> bool {
    chars > limit || from.is_some_and(|f| sent >= f)
}

/// `sys:read_file` answering [`file`] for each path; keeps the paths it ran.
struct Files(Arc<Mutex<Vec<String>>>);

impl Tools for Files {
    fn hint(&self) -> String {
        "TOOLS: @tool sys:read_file {\"path\": \u{2026}}".into()
    }
    fn call(&self, _: &str, _: &str, args: &str) -> Result<String, String> {
        let v: serde_json::Value = serde_json::from_str(args).map_err(|e| e.to_string())?;
        let path = v["path"].as_str().unwrap_or_default().to_string();
        self.0.lock().unwrap().push(path.clone());
        Ok(file(&path))
    }
    fn repeatable(&self, _: &str, _: &str) -> bool {
        true
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "sys".into(),
            tool: "read_file".into(),
            description: "read a file".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }]
    }
}

/// 5,500 chars for `c.rs`, 3,000 for any other path, in numbered lines.
pub(super) fn file(path: &str) -> String {
    let size = if path == "c.rs" { 5_500 } else { 3_000 };
    let mut lines: Vec<String> = (0..size / 50 - 1)
        .map(|i| format!("{path} line {i:03}: {}", "z".repeat(49 - path.len() - 11)))
        .collect();
    lines.push(format!("END OF {path}"));
    let s = lines.join("\n");
    format!("{s}{}", " ".repeat(size - s.chars().count()))
}

fn ctx(bus: &EventBus) -> AgentContext {
    AgentContext {
        cancel: Default::default(),
        // A pool of two tasks: eight rounds, room for reads after the cut.
        budget: crate::tools::budget::ToolBudget::for_run(2),
        agent: AgentId(4),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "what is in a.rs, b.rs and c.rs?".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    }
}

/// Run one task on `provider` with the file surface: its result, the reads
/// that ran, and how many times the pane was told the context was full.
pub(super) async fn run_on(provider: Arc<dyn Provider>) -> (TaskResult, Vec<String>, usize) {
    let ran = Arc::new(Mutex::new(Vec::new()));
    let bus = EventBus::new(512);
    let mut rx = bus.subscribe();
    let out = crate::apiagent::ApiAgent::new(provider, 256)
        .with_tools(Arc::new(Files(Arc::clone(&ran))))
        .run(ctx(&bus))
        .await;
    let mut notes = 0;
    while let Ok(ev) = rx.try_recv() {
        if let HiveEvent::OutputDelta { text, .. } = ev {
            notes += usize::from(text.trim() == CONTEXT_FULL);
        }
    }
    let ran = ran.lock().unwrap().clone();
    (out, ran, notes)
}
