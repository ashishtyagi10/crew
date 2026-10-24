//! A swarm worker's failed check, read by the advisor (`crate::advisor`): the
//! advice joins the worker's next prompt, its line is said once, and the read
//! is billed on the run's bus under the worker.
use std::sync::{Arc, Mutex};

use super::ApiAgent;
use crate::advisor::{Ask, Asked, HEAD};
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crate::spent::Spent;
use crate::tools::Tools;

/// Builds (and fails), then answers; keeps every prompt.
struct Worker(Mutex<u32>, Arc<Mutex<Vec<String>>>);

impl Provider for Worker {
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
            1 => "@tool sys:run {\"cmd\": \"cargo build\"}".to_string(),
            _ => "fixed the call site".to_string(),
        };
        Box::pin(async move {
            Ok(Completion {
                text,
                ..Default::default()
            })
        })
    }
}

/// `sys:run` failing the build on exit 101, as the session's surface does.
struct Builds;

impl Tools for Builds {
    fn hint(&self) -> String {
        "TOOLS: sys:run".into()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        Ok("exit 101\nerror[E0308]: mismatched types\n  --> src/a.rs:3:5".into())
    }
    fn failed(&self, s: &str, t: &str, out: &str) -> bool {
        (s, t) == ("sys", "run") && out.starts_with("exit 101")
    }
}

#[tokio::test]
async fn a_workers_failed_build_is_advised_and_billed() {
    let prompts: Arc<Mutex<Vec<String>>> = Arc::default();
    let said: Arc<Mutex<Vec<String>>> = Arc::default();
    let ask: Ask = Arc::new(|_brief: String| -> Asked {
        Box::pin(async {
            let spent = Spent {
                input: 40,
                output: 9,
                micros_usd: 120,
            };
            Ok((
                "E0308 at src/a.rs:3 \u{2014} fix the call site".to_string(),
                spent,
            ))
        })
    });
    let keep = Arc::clone(&said);
    let agent = ApiAgent::new(
        Arc::new(Worker(Mutex::default(), Arc::clone(&prompts))),
        256,
    )
    .with_tools(Arc::new(Builds))
    .with_advisor(
        Arc::new(move || Some(Arc::clone(&ask))),
        Arc::new(move |l| keep.lock().unwrap().push(l.to_string())),
    );
    let bus = EventBus::new(256);
    let mut rx = bus.subscribe();
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(7),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "make it build".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    let out = agent.run(ctx).await;
    assert!(out.success, "{}", out.output);
    let prompts = prompts.lock().unwrap();
    assert_eq!(prompts.len(), 2);
    assert!(
        prompts[1].contains(&format!("{HEAD}\n- E0308 at src/a.rs:3")),
        "{}",
        prompts[1]
    );
    assert_eq!(
        *said.lock().unwrap(),
        ["advised \u{b7} E0308 at src/a.rs:3 \u{2014} fix the call site"]
    );
    let mut billed = (0, 0);
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::TokenDelta {
                agent: AgentId(7),
                input,
                ..
            } => billed.0 += input,
            HiveEvent::CostDelta {
                agent: AgentId(7),
                micros_usd,
            } => billed.1 += micros_usd,
            _ => {}
        }
    }
    assert_eq!(billed, (40, 120), "the read, on the worker's bill");
}
