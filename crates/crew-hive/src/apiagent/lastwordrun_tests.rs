use super::*;
use crate::agent::Agent;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError};
use crate::tools::{budget::ToolBudget, ToolSpec, Tools, MAX_TOOL_ROUNDS};
use std::sync::Mutex;

const HINT: &str = "TOOLS: @tool weather:current";
const NOTE: &str =
    "[tool budget spent \u{2014} 4 calls for this run; the last request was not run]";

/// Asks for a tool on every call that offers one (schemas on the wire, or the
/// text hint in the prompt) until `asks` runs out, then answers "done early".
/// A call that offers none gets `last`, or an error when that is `None`.
struct Rule {
    native: bool,
    asks: Mutex<u32>,
    last: Option<&'static str>,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
}

type Reply =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>>;

impl Provider for Rule {
    fn supports_tools(&self) -> bool {
        self.native
    }
    fn complete(&self, req: CompletionRequest) -> Reply {
        let offered = !req.tools.is_empty() || req.prompt.contains(HINT);
        self.seen.lock().unwrap().push(req);
        let mut asks = self.asks.lock().unwrap();
        let said = |text: &str| {
            Ok(Completion {
                text: text.into(),
                input_tokens: 1,
                output_tokens: 1,
                ..Default::default()
            })
        };
        let reply = match (offered, *asks > 0) {
            (true, true) if self.native => {
                *asks -= 1;
                let call = ToolInvocation {
                    id: format!("c{asks}"),
                    name: "weather__current".into(),
                    input: serde_json::json!({ "q": *asks }),
                };
                said("").map(|c| Completion {
                    calls: vec![call],
                    ..c
                })
            }
            (true, true) => {
                *asks -= 1;
                said("reading more\n@tool weather:current {}")
            }
            (true, false) => said("done early"),
            (false, _) => self
                .last
                .map_or_else(|| Err(ProviderError::Http("reset".into())), said),
        };
        Box::pin(async move { reply })
    }
}

struct Weather;

impl Tools for Weather {
    fn hint(&self) -> String {
        HINT.into()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        Ok("Oslo 4C".into())
    }
    fn specs(&self) -> Vec<ToolSpec> {
        let schema = serde_json::json!({ "type": "object" });
        vec![ToolSpec {
            server: "weather".into(),
            tool: "current".into(),
            description: "now".into(),
            input_schema: schema,
        }]
    }
}

/// One task on a run pool with two of its four rounds already drawn: its
/// output, every request the provider saw, and how many calls were billed.
async fn run(
    native: bool,
    asks: u32,
    last: Option<&'static str>,
) -> (String, Vec<CompletionRequest>, usize) {
    let seen: Arc<Mutex<Vec<_>>> = Arc::default();
    let (asks, log) = (Mutex::new(asks), Arc::clone(&seen));
    let provider = Arc::new(Rule {
        native,
        asks,
        last,
        seen: log,
    });
    let budget = ToolBudget::solo();
    for used in 0..MAX_TOOL_ROUNDS - 2 {
        budget.take(used);
    }
    let bus = EventBus::new(256);
    let mut rx = bus.subscribe();
    let task = TaskSpec {
        id: TaskId(1),
        title: "t".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: "weather?".into(),
        specialty: String::new(),
        expertise: String::new(),
    };
    let ctx = AgentContext {
        cancel: Default::default(),
        budget,
        agent: AgentId(1),
        task,
        deps: vec![],
        bus: bus.clone(),
    };
    let agent = super::super::ApiAgent::new(provider, 256).with_tools(Arc::new(Weather));
    let out = agent.run(ctx).await.output;
    let mut billed = 0;
    while let Ok(ev) = rx.try_recv() {
        billed += usize::from(matches!(ev, HiveEvent::TokenDelta { .. }));
    }
    let seen = seen.lock().unwrap().clone();
    (out, seen, billed)
}

/// Two rounds run, the third ask is refused, and one more call with the
/// tools taken away turns what was gathered into the answer.
#[tokio::test]
async fn a_worker_the_budget_stops_answers_from_what_it_gathered() {
    for native in [true, false] {
        let (out, seen, billed) = run(native, u32::MAX, Some("the answer is 42")).await;
        // Two rounds, the refused ask, then the last word.
        assert_eq!(seen.len(), 4, "native {native}");
        let offered = |r: &CompletionRequest| !r.tools.is_empty() || r.prompt.contains(HINT);
        assert!(seen[..3].iter().all(offered), "native {native}");
        let last = &seen[3];
        assert!(
            last.tools.is_empty(),
            "native {native}: tools on the last word"
        );
        assert!(
            !last.prompt.contains(HINT),
            "native {native}: {}",
            last.prompt
        );
        // Tool turns with no tools beside them are a request Anthropic refuses.
        assert!(last.turns.is_empty(), "native {native}");
        let refused = "RESULT:\nnot run \u{2014} tool budget spent";
        let carried = ["weather?", "RESULT:\nOslo 4C", refused, INSTRUCTION];
        assert!(
            carried.iter().all(|s| last.prompt.contains(s)),
            "{}",
            last.prompt
        );
        assert_eq!(
            out,
            format!("the answer is 42\n\n{NOTE}"),
            "native {native}"
        );
        assert_eq!(billed, 4, "native {native}: the last word is billed too");
    }
}

/// A last word that fails leaves today's output: the native ask wrote
/// nothing, so the note alone; the text ask wrote a line above its call.
#[tokio::test]
async fn a_failed_last_word_keeps_the_output_it_had() {
    let (out, seen, billed) = run(true, u32::MAX, None).await;
    assert_eq!((seen.len(), billed), (4, 3));
    assert_eq!(out, NOTE);
    let (out, seen, _) = run(false, u32::MAX, None).await;
    assert_eq!(seen.len(), 4);
    assert_eq!(out, format!("reading more\n\n{NOTE}"));
    let (out, _, _) = run(true, u32::MAX, Some("  \n")).await;
    assert_eq!(out, NOTE, "an empty last word is no answer");
}

/// A worker whose last reply answered is left alone: no extra call.
#[tokio::test]
async fn a_worker_that_answered_within_budget_makes_no_extra_call() {
    for native in [true, false] {
        let (out, seen, billed) = run(native, 1, Some("unused")).await;
        assert_eq!(out, "done early", "native {native}");
        assert_eq!((seen.len(), billed), (2, 2), "native {native}");
    }
}
