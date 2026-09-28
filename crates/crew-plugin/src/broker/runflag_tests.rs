//! A command that ran and failed is a FAILED call on both engines, through
//! the surface a live session builds — a real `/bin/sh`, not a fake result.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::broker::adapter::{HopStream, Usage};
use crate::broker::hop::RunStats;
use crate::broker::session::sessiontest::sys_surface;
use crate::broker::{Adapter, Broker, Envelope};
use crate::Registry;

const FAILING: &str = "echo x; exit 3";

/// Answers once the tool result is back, and keeps what it was shown.
struct Agent(Mutex<Vec<String>>);

impl Adapter for Agent {
    fn name(&self) -> &str {
        "planner"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.0.lock().unwrap().push(body.to_string());
        Ok("it exited 3".into())
    }
}

#[test]
fn the_relay_shows_a_non_zero_exit_as_a_failed_card() {
    let broker =
        Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5)).with_tools(sys_surface());
    let agent = Agent(Mutex::new(Vec::new()));
    let mut hops = Vec::new();
    broker.run_tools(
        &agent,
        "task",
        format!("@tool sys:run {{\"cmd\": \"{FAILING}\"}}"),
        &mut RunStats::default(),
        &mut Usage::default(),
        &Envelope::new("user", "planner", "t1", "task"),
        &HopStream::noop(),
        &mut |h| hops.push(h),
    );
    let card = hops
        .iter()
        .find(|h| h.from == "sys:run")
        .map(|h| h.text.clone())
        .expect("a result card");
    assert!(card.starts_with("[tool] sys:run \u{2717} "), "{card}");
    // Only the flag changed: the agent still reads what the command said.
    let seen = agent.0.lock().unwrap().concat();
    assert!(seen.contains("RESULT:\nexit 3\nx\n"), "{seen}");
}

type Reply = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<crew_hive::Completion, crew_hive::ProviderError>>
            + Send,
    >,
>;

/// A native-tools model: calls `sys:run` once, then answers, keeping every
/// tool outcome it was sent.
struct Native(Arc<Mutex<Vec<crew_hive::ToolOutcome>>>);

impl crew_hive::Provider for Native {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(&self, req: crew_hive::CompletionRequest) -> Reply {
        let mut done = false;
        for turn in req.turns {
            if let crew_hive::Turn::ToolResults(r) = turn {
                self.0.lock().unwrap().extend(r);
                done = true;
            }
        }
        let calls = match done {
            true => vec![],
            false => vec![crew_hive::ToolInvocation {
                id: "c1".into(),
                name: "sys__run".into(),
                input: serde_json::json!({ "cmd": FAILING }),
                bad_args: None,
            }],
        };
        let text = if done { "it exited 3" } else { "" }.to_string();
        Box::pin(async move {
            Ok(crew_hive::Completion {
                text,
                calls,
                input_tokens: 1,
                output_tokens: 1,
                ..Default::default()
            })
        })
    }
}

#[test]
fn the_swarm_sends_a_non_zero_exit_to_the_provider_as_an_error() {
    let outcomes = Arc::new(Mutex::new(Vec::new()));
    let bus = crew_hive::EventBus::new(64);
    let mut rx = bus.subscribe();
    let ctx = crew_hive::AgentContext {
        cancel: Default::default(),
        budget: crew_hive::ToolBudget::solo(),
        agent: crew_hive::AgentId(1),
        task: crew_hive::TaskSpec {
            id: crew_hive::TaskId(1),
            title: "build".into(),
            agent: crew_hive::AgentKind::Api { system: None },
            model: crew_hive::ModelTier::Standard,
            deps: vec![],
            prompt: "run the build".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    let agent = crew_hive::ApiAgent::new(Arc::new(Native(Arc::clone(&outcomes))), 256)
        .with_tools(sys_surface());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(crew_hive::Agent::run(&agent, ctx));
    let sent = outcomes.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(
        sent[0].is_error,
        "a failed build went back as data: {sent:?}"
    );
    assert!(sent[0].content.starts_with("exit 3\nx\n"), "{sent:?}");
    // …and the pane's line for it is the failed one too.
    let mut oks = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let crew_hive::HiveEvent::ToolResult { ok, .. } = ev {
            oks.push(ok);
        }
    }
    assert_eq!(oks, vec![false]);
}
