//! What the before-the-answer tests share: a model that plays scripted
//! `sys:` calls then answers, a surface that edits and runs, and one whole
//! turn on them with the real advisor.
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::super::*;
use crate::advisor::{Advisor, Ask, Asked, Find, Hooks};
use crate::bus::{AgentId, EventBus};
use crate::provider::{Completion, CompletionRequest, ProviderError, ToolInvocation, Turn};
use crate::spent::Spent;
use crate::tools::budget::ToolBudget;
use crate::tools::ToolSpec;

pub(super) const ADVICE: &str = "run cargo test -p a before you answer";

/// One scripted call: `sys:<tool>` with its JSON arguments.
pub(super) type Step = (&'static str, &'static str);

/// Makes `steps` one per round, then answers `done`; keeps each request as
/// the model would read it (prompt and every turn, flattened).
struct Plays {
    native: bool,
    steps: Mutex<Vec<Step>>,
    shown: Arc<Mutex<Vec<String>>>,
}

impl Provider for Plays {
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
        let c = match (self.steps.lock().unwrap().pop(), self.native) {
            (None, _) => Completion {
                text: "done".into(),
                ..Default::default()
            },
            (Some((tool, args)), false) => Completion {
                text: format!("@tool sys:{tool} {args}"),
                ..Default::default()
            },
            (Some((tool, args)), true) => Completion {
                calls: vec![ToolInvocation {
                    id: format!("c-{tool}"),
                    name: format!("sys__{tool}"),
                    input: serde_json::from_str(args).unwrap(),
                    bad_args: None,
                }],
                ..Default::default()
            },
        };
        Box::pin(async move { Ok(c) })
    }
}

/// `sys:edit` that always applies, and `sys:run` whose command fails when
/// it says `fail`, the way the session's surface reports an exit.
struct Shop;

impl Tools for Shop {
    fn hint(&self) -> String {
        "TOOLS: sys:edit, sys:run".into()
    }
    fn call(&self, _: &str, tool: &str, args: &str) -> Result<String, String> {
        Ok(match (tool, args.contains("fail")) {
            ("run", true) => format!("exit 101\nerror: {args}"),
            ("run", false) => "exit 0\nok".into(),
            _ => "edited".into(),
        })
    }
    fn failed(&self, _: &str, tool: &str, out: &str) -> bool {
        tool == "run" && out.starts_with("exit 101")
    }
    fn specs(&self) -> Vec<ToolSpec> {
        ["edit", "run"]
            .map(|t| ToolSpec {
                server: "sys".into(),
                tool: t.into(),
                description: t.into(),
                input_schema: serde_json::json!({"type": "object"}),
            })
            .into()
    }
}

/// What a turn playing `steps` did: the requests the model saw, the briefs
/// the advisor got, and smith's lines.
pub(super) async fn turn(native: bool, steps: &[Step]) -> (Vec<String>, Vec<String>, Vec<String>) {
    turn_with(native, steps, true).await
}

/// [`turn`], with an advisor to find only when `configured`.
pub(super) async fn turn_with(
    native: bool,
    steps: &[Step],
    configured: bool,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let shown: Arc<Mutex<Vec<String>>> = Arc::default();
    let (briefs, said): (Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<String>>>) = Default::default();
    let keep = Arc::clone(&briefs);
    let find: Find = Arc::new(move || {
        let keep = Arc::clone(&keep);
        configured.then_some(())?;
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
    let tools: Arc<dyn Tools> = Arc::new(Shop);
    let advisor = Advisor::new("fix a.rs", Arc::clone(&tools), find, hooks);
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::new(EventBus::new(256)),
        budget: ToolBudget::turn(8),
        cancel: Arc::new(AtomicBool::new(false)),
        advice: Some(Arc::new(advisor)),
    };
    let spec = TurnSpec {
        effort: None,
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
    let model = Arc::new(Plays {
        native,
        steps: Mutex::new(steps.iter().rev().copied().collect()),
        shown: Arc::clone(&shown),
    });
    let ended = run(ctx, spec, model, Some(tools), None).await;
    assert_eq!(ended, Ended::Answered("done".into()));
    let out = |m: &Arc<Mutex<Vec<String>>>| m.lock().unwrap().clone();
    (out(&shown), out(&briefs), out(&said))
}
