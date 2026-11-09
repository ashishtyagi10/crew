//! The advisor on a whole turn that stops making progress (`advisor::stall`):
//! a scripted model re-reading one file is advised once and the advice is in
//! the next request; one reading something new each round is never advised.
//! Both engines, since each feeds the advice back its own way.
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::*;
use crate::advisor::{Advisor, Ask, Asked, Find, Hooks, HEAD};
use crate::bus::{AgentId, EventBus};
use crate::provider::{Completion, CompletionRequest, ProviderError, ToolInvocation, Turn};
use crate::spent::Spent;
use crate::tools::budget::ToolBudget;
use crate::tools::ToolSpec;

const ADVICE: &str = "you have a.rs already: edit it or answer";

/// Reads `paths` one per round, then answers; keeps each request as the
/// model would read it (prompt and every turn, flattened).
struct Reads {
    native: bool,
    paths: Mutex<Vec<&'static str>>,
    shown: Arc<Mutex<Vec<String>>>,
}

impl Provider for Reads {
    fn supports_tools(&self) -> bool {
        self.native
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let turns = req.turns.iter().map(|t| match t {
            Turn::User(s) => s.clone(),
            other => format!("{other:?}"),
        });
        let flat = std::iter::once(req.prompt.clone()).chain(turns).collect();
        self.shown.lock().unwrap().push(flat);
        let next = self.paths.lock().unwrap().pop();
        let c = match (next, self.native) {
            (None, _) => Completion {
                text: "done".into(),
                ..Default::default()
            },
            (Some(p), false) => Completion {
                text: format!("@tool fs:read {{\"path\": \"{p}\"}}"),
                ..Default::default()
            },
            (Some(p), true) => Completion {
                calls: vec![ToolInvocation {
                    id: format!("c-{p}"),
                    name: "fs__read".into(),
                    input: serde_json::json!({ "path": p }),
                    bad_args: None,
                }],
                ..Default::default()
            },
        };
        Box::pin(async move { Ok(c) })
    }
}

/// A file per path, the same text each time it is read.
struct Files;

impl Tools for Files {
    fn hint(&self) -> String {
        "TOOLS: fs:read".into()
    }
    fn call(&self, _: &str, _: &str, args: &str) -> Result<String, String> {
        Ok(format!("contents of {args}"))
    }
    fn repeatable(&self, _: &str, _: &str) -> bool {
        true
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "fs".into(),
            tool: "read".into(),
            description: "read a file".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }]
    }
}

/// What a turn reading `paths` in order did: the requests the model saw, the
/// briefs the advisor got, and smith's lines.
async fn turn(native: bool, paths: &[&'static str]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let shown: Arc<Mutex<Vec<String>>> = Arc::default();
    let (briefs, said): (Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<String>>>) = Default::default();
    let keep = Arc::clone(&briefs);
    let find: Find = Arc::new(move || {
        let keep = Arc::clone(&keep);
        Some(Arc::new(move |brief: String| -> Asked {
            keep.lock().unwrap().push(brief);
            Box::pin(async { Ok((ADVICE.to_string(), Spent::default())) })
        }) as Ask)
    });
    let says = Arc::clone(&said);
    let hooks = Hooks {
        say: Arc::new(move |l| says.lock().unwrap().push(l.to_string())),
        spent: Arc::new(|_| {}),
    };
    let tools: Arc<dyn Tools> = Arc::new(Files);
    let advisor = Advisor::new("fix a.rs", Arc::clone(&tools), find, hooks);
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::new(EventBus::new(256)),
        budget: ToolBudget::turn(8),
        cancel: Arc::new(AtomicBool::new(false)),
        advice: Some(Arc::new(advisor)),
    };
    let spec = TurnSpec {
        ask: "fix a.rs".into(),
        prompt: "TASK: fix a.rs".into(),
        hinted: None,
        shed: None,
        ending: None,
        call_timeout: None,
        system: None,
        model: "m".into(),
        tier: ModelTier::Standard,
        max_tokens: 64,
    };
    let model = Arc::new(Reads {
        native,
        paths: Mutex::new(paths.iter().rev().copied().collect()),
        shown: Arc::clone(&shown),
    });
    let ended = run(ctx, spec, model, Some(tools), None).await;
    assert_eq!(ended, Ended::Answered("done".into()));
    let out = |m: &Arc<Mutex<Vec<String>>>| m.lock().unwrap().clone();
    (out(&shown), out(&briefs), out(&said))
}

#[tokio::test]
async fn a_turn_re_reading_one_file_is_advised_once_and_reads_the_advice() {
    for native in [false, true] {
        let (shown, briefs, said) = turn(native, &["a.rs", "a.rs", "a.rs", "a.rs"]).await;
        assert_eq!(briefs.len(), 1, "native {native}: one read per stall");
        assert!(briefs[0].contains("told it nothing new"), "{}", briefs[0]);
        assert!(briefs[0].contains("2. fs:read"), "{}", briefs[0]);
        assert_eq!(
            said,
            [format!("advised \u{b7} {ADVICE}")],
            "native {native}"
        );
        // Request 3 follows round 2, the first repeat; 1 and 2 carry none.
        assert!(!shown[1].contains(HEAD), "native {native}: {}", shown[1]);
        let advised = &shown[2];
        assert!(
            advised.contains(HEAD) && advised.contains(ADVICE),
            "{advised}"
        );
    }
}

#[tokio::test]
async fn a_turn_reading_something_new_each_round_is_never_advised() {
    for native in [false, true] {
        let (shown, briefs, said) = turn(native, &["a.rs", "b.rs", "c.rs"]).await;
        assert!(briefs.is_empty() && said.is_empty(), "native {native}");
        assert!(shown.iter().all(|s| !s.contains(HEAD)), "native {native}");
    }
}
