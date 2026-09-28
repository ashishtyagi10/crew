//! A reply's reads run together; a reply with a write in it runs in order.
//! Each test drives the whole agent against a runner whose every call sleeps,
//! so the clock says which way the batch ran.
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{
    Completion, CompletionRequest, Provider, ProviderError, ToolInvocation, Turn,
};
use crate::tools::{ToolSpec, Tools};
use std::sync::Arc;

/// Replays `replies`, keeping every request.
struct Scripted(Mutex<Vec<Completion>>, Arc<Mutex<Vec<CompletionRequest>>>);

impl Provider for Scripted {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.1.lock().unwrap().push(req);
        let c = self.0.lock().unwrap().pop().unwrap_or_default();
        Box::pin(async move { Ok(c) })
    }
}

/// `fs:read` only looks, `fs:write` does not; every call sleeps [`NAP`] and
/// is recorded as it STARTS, and answers with its own arguments.
struct Sleepy(Arc<Mutex<Vec<String>>>);

const NAP: Duration = Duration::from_millis(300);

impl Tools for Sleepy {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.0.lock().unwrap().push(format!("{tool} {args}"));
        std::thread::sleep(NAP);
        Ok(format!("{tool} of {args}"))
    }
    fn repeatable(&self, _server: &str, tool: &str) -> bool {
        tool == "read"
    }
    fn specs(&self) -> Vec<ToolSpec> {
        ["read", "write"]
            .map(|t| ToolSpec {
                server: "fs".into(),
                tool: t.into(),
                description: String::new(),
                input_schema: serde_json::json!({"type": "object"}),
            })
            .to_vec()
    }
}

/// What one batch did: how long the run took, the calls in the order they
/// started, the `(id, content)` results the follow-up carried, and the tool
/// events the bus saw, `call`/`result` each.
struct Ran {
    took: Duration,
    ran: Vec<String>,
    results: Vec<(String, String)>,
    events: Vec<&'static str>,
}

/// One reply asking for `(tool, path)` in order, then an answer.
async fn batch(asks: &[(&str, &str)]) -> Ran {
    let calls = asks
        .iter()
        .enumerate()
        .map(|(i, (tool, path))| ToolInvocation {
            id: format!("id{i}"),
            name: format!("fs__{tool}"),
            input: serde_json::json!({ "path": path }),
        })
        .collect();
    let replies = vec![
        Completion {
            text: "done".into(),
            ..Default::default()
        },
        Completion {
            calls,
            ..Default::default()
        },
    ];
    let seen = Arc::new(Mutex::new(Vec::new()));
    let ran = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(Scripted(Mutex::new(replies), Arc::clone(&seen)));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(1),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "read them".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    let started = Instant::now();
    let out = crate::apiagent::ApiAgent::new(provider, 256)
        .with_tools(Arc::new(Sleepy(Arc::clone(&ran))))
        .run(ctx)
        .await;
    let took = started.elapsed();
    assert_eq!(out.output, "done");
    let seen = seen.lock().unwrap();
    let results = match &seen[1].turns[1] {
        Turn::ToolResults(r) => r.iter().map(|o| (o.id.clone(), o.content.clone())),
        other => panic!("{other:?}"),
    };
    let mut events = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::ToolCall { .. } => events.push("call"),
            HiveEvent::ToolResult { .. } => events.push("result"),
            _ => {}
        }
    }
    let ran = ran.lock().unwrap().clone();
    Ran {
        took,
        ran,
        results: results.collect(),
        events,
    }
}

#[tokio::test]
async fn three_reads_in_one_reply_run_at_once_and_answer_in_order() {
    let Ran {
        took, ran, results, ..
    } = batch(&[("read", "a"), ("read", "b"), ("read", "c")]).await;
    assert_eq!(ran.len(), 3);
    // Together is one nap; in turn is three (900 ms).
    assert!(took < NAP * 2 + Duration::from_millis(200), "took {took:?}");
    let want: Vec<(String, String)> = ["a", "b", "c"]
        .iter()
        .enumerate()
        .map(|(i, p)| (format!("id{i}"), format!("read of {{\"path\":\"{p}\"}}")))
        .collect();
    assert_eq!(results, want);
}

#[tokio::test]
async fn a_write_among_the_reads_runs_the_reply_in_the_order_written() {
    let asks = [("read", "a"), ("write", "a"), ("read", "a")];
    let Ran {
        took, ran, results, ..
    } = batch(&asks).await;
    let want: Vec<String> = asks
        .iter()
        .map(|(t, p)| format!("{t} {{\"path\":\"{p}\"}}"))
        .collect();
    assert_eq!(ran, want, "started in the order asked");
    assert!(took >= NAP * 3, "took {took:?}");
    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["id0", "id1", "id2"]);
}

/// Each call still gets its own `ToolCall` and `ToolResult` on the bus, and
/// a read asked twice in one reply runs once.
#[tokio::test]
async fn a_read_asked_twice_in_one_batch_runs_once() {
    let r = batch(&[("read", "a"), ("read", "b"), ("read", "a")]).await;
    assert_eq!(r.ran.len(), 2, "{:?}", r.ran);
    assert!(
        r.results[2].1.contains("same call as round 1"),
        "{:?}",
        r.results
    );
    assert_eq!(
        r.events,
        ["call", "call", "call", "result", "result", "result"]
    );
}
