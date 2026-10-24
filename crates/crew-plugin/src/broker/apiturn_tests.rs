//! A relay hop of an API agent with tools, run on the harness: driven whole
//! through `Broker::run`, a scripted model behind a real `ApiAdapter`.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crew_hive::provider::ToolInvocation;
use crew_hive::{Completion, CompletionRequest, HiveEvent, Provider, ProviderError, ToolSpec};

use crate::broker::adapter::Adapter;
use crate::broker::apiadapter::ApiAdapter;
use crate::broker::hop::{Hop, HopKind};
use crate::broker::toolcall::ToolRunner;
use crate::broker::Broker;
use crate::{PluginEvent, Registry};

type Reply = Result<Completion, ProviderError>;

/// A model that answers from a script, one reply a call, and keeps every
/// request it was sent. `native` is whether it speaks tool-use.
struct Model {
    native: bool,
    replies: Mutex<Vec<Reply>>,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl Provider for Model {
    fn supports_tools(&self) -> bool {
        self.native
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Reply> + Send>> {
        self.seen.lock().unwrap().push(req);
        let mut replies = self.replies.lock().unwrap();
        let reply = match replies.is_empty() {
            true => Ok(said("@done")),
            false => replies.remove(0),
        };
        Box::pin(async move { reply })
    }
    /// Streams the reply's text before it lands, as a real provider does.
    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: crew_hive::ChunkFn,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Reply> + Send>> {
        let call = self.complete(req);
        Box::pin(async move {
            let reply = call.await;
            if let Ok(c) = &reply {
                on_chunk(crew_hive::Chunk::Text(&c.text));
            }
            reply
        })
    }
}

fn said(text: &str) -> Completion {
    Completion {
        text: text.into(),
        input_tokens: 100,
        output_tokens: 10,
        ..Default::default()
    }
}

/// A native reply that calls `name` with `input`.
fn calling(name: &str, input: serde_json::Value) -> Completion {
    Completion {
        calls: vec![ToolInvocation {
            id: format!("id-{name}"),
            name: name.into(),
            input,
            bad_args: None,
        }],
        ..said("")
    }
}

/// `fs:read`, answered with the path it was asked for; `sys:todo`, accepted
/// unless its list is empty. Every call is kept.
struct Files(Arc<Mutex<Vec<String>>>);

impl ToolRunner for Files {
    fn hint(&self) -> String {
        "TOOLS: @tool fs:read {\"path\": \u{2026}}".into()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.0
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        match (server, tool) {
            ("sys", "todo") if args.contains("\"items\":[]") => Err("an empty list".into()),
            ("sys", "todo") => Ok("checklist kept".into()),
            _ => Ok(format!("contents of {args}")),
        }
    }
    fn specs(&self) -> Vec<ToolSpec> {
        let spec = |server: &str, tool: &str| ToolSpec {
            server: server.into(),
            tool: tool.into(),
            description: "reads".into(),
            input_schema: serde_json::json!({ "type": "object" }),
        };
        vec![spec("fs", "read"), spec("sys", "todo")]
    }
}

/// What one hop did: its hops, the events it streamed, the requests the
/// model saw, the tools it ran, and the thread's stats.
struct Turn {
    hops: Vec<Hop>,
    events: Vec<PluginEvent>,
    seen: Vec<CompletionRequest>,
    ran: Vec<String>,
    stats: crate::broker::RunStats,
}

impl Turn {
    fn done(&self) -> Option<&str> {
        self.hops
            .iter()
            .find(|h| h.kind == HopKind::Done)
            .map(|h| h.text.as_str())
    }
    fn errors(&self) -> Vec<&str> {
        self.hops
            .iter()
            .filter(|h| h.kind == HopKind::Error)
            .map(|h| h.text.as_str())
            .collect()
    }
    fn tool_events(&self) -> Vec<&HiveEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                PluginEvent::Hive { event } => Some(event),
                _ => None,
            })
            .collect()
    }
}

/// One user message to `coder` alone, an `ApiAdapter` on `replies`, with
/// [`Files`] attached; `rounds` tool rounds, `stop` its `/stop`.
fn hop(native: bool, replies: Vec<Reply>, rounds: u32, stop: Arc<AtomicBool>) -> Turn {
    let seen: Arc<Mutex<Vec<CompletionRequest>>> = Arc::default();
    let model = Model {
        native,
        replies: Mutex::new(replies),
        seen: Arc::clone(&seen),
    };
    let agent = ApiAdapter::new("coder", "m", "", None, Arc::new(model)).unwrap();
    let ran: Arc<Mutex<Vec<String>>> = Arc::default();
    let broker = Broker::new(
        Registry::new(vec![Box::new(agent) as Box<dyn Adapter>]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Files(Arc::clone(&ran))))
    .with_tool_rounds(rounds)
    .with_cancel_flag(stop);
    let events: Arc<Mutex<Vec<PluginEvent>>> = Arc::default();
    let keep = Arc::clone(&events);
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> =
        Arc::new(move |e| keep.lock().unwrap().push(e));
    let mut hops = Vec::new();
    let stats = broker.run("user", "coder", "read a.rs", "t1", &emit, &mut |h| {
        hops.push(h)
    });
    let events = events.lock().unwrap().clone();
    let seen = seen.lock().unwrap().clone();
    let ran = ran.lock().unwrap().clone();
    Turn {
        hops,
        events,
        seen,
        ran,
        stats,
    }
}

fn go(native: bool, replies: Vec<Reply>) -> Turn {
    hop(native, replies, 4, Arc::default())
}

/// A provider that speaks tool-use gets the tools on the wire, each with its
/// schema, and no `@tool` hint beside them; the result goes back as a turn,
/// and the answer routes as the hop's.
#[test]
fn a_native_provider_gets_its_tools_on_the_wire() {
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            Ok(said("a.rs holds main\n@done")),
        ],
    );
    assert_eq!(t.ran, vec!["fs:read {\"path\":\"a.rs\"}"]);
    assert_eq!(t.seen.len(), 2);
    assert!(!t.seen[0].tools.is_empty(), "tools on the wire");
    assert!(!t.seen[0].prompt.contains("TOOLS:"), "{}", t.seen[0].prompt);
    assert!(t.seen[0].prompt.contains("HOW TO REPLY"), "the frame");
    assert_eq!(t.seen[1].turns.len(), 2, "the call, then its result");
    assert_eq!(t.done(), Some("a.rs holds main"));
    assert!(t.errors().is_empty(), "{:?}", t.errors());
}

/// Without tool-use the hint sits where the relay's frame puts it, inside
/// TASK and above HOW TO REPLY, and every follow-up re-states it.
#[test]
fn a_text_provider_reads_the_hint_inside_the_task() {
    let t = go(
        false,
        vec![
            Ok(said("looking\n@tool fs:read {\"path\": \"a.rs\"}")),
            Ok(said("a.rs holds main\n@done")),
        ],
    );
    assert_eq!(t.ran.len(), 1, "{:?}", t.ran);
    for p in t.seen.iter().map(|r| &r.prompt) {
        let (hint, how) = (p.find("TOOLS:").unwrap(), p.find("HOW TO REPLY").unwrap());
        assert!(hint < how, "{p}");
        assert_eq!(p.matches("TOOLS:").count(), 1, "{p}");
    }
    let follow = &t.seen[1].prompt;
    assert!(follow.contains("RESULT:\ncontents of"), "{follow}");
    assert!(follow.contains("You may make 3 more round(s)"), "{follow}");
    assert_eq!(t.done(), Some("a.rs holds main"));
}

/// The pane sees the turn the way it sees Claude Code's: each call and its
/// result on the tool lane under the agent's name, the reply streamed, its
/// tokens ticked; one Dialing hop for the hop, not one a round.
#[test]
fn the_pane_gets_the_tool_lane_and_the_stream() {
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            Ok(said("a.rs holds main\n@done")),
        ],
    );
    let lane = t.tool_events();
    assert!(matches!(lane[0], HiveEvent::ToolCall { label, .. } if label == "fs:read"));
    assert!(matches!(lane[1], HiveEvent::ToolResult { label, ok: true, .. } if label == "fs:read"));
    let dials = t.hops.iter().filter(|h| h.kind == HopKind::Dialing).count();
    assert_eq!(dials, 1);
    let ticked = t
        .events
        .iter()
        .any(|e| matches!(e, PluginEvent::StatsTick { .. }));
    assert!(ticked, "{:?}", t.events);
}

/// An accepted checklist goes out as the plan and makes no tool line; a
/// refused one changed nothing, so nothing goes out for it.
#[test]
fn a_checklist_write_goes_out_as_the_plan_only_when_accepted() {
    let todo =
        |items: serde_json::Value| calling("sys__todo", serde_json::json!({ "items": items }));
    let step = serde_json::json!([{"text": "read a.rs", "status": "pending"}]);
    let t = go(true, vec![Ok(todo(step)), Ok(said("planned\n@done"))]);
    let plans = t
        .events
        .iter()
        .filter(|e| matches!(e, PluginEvent::Todo { .. }))
        .count();
    assert_eq!(plans, 1, "{:?}", t.events);
    assert!(
        t.tool_events().is_empty(),
        "no tool line: {:?}",
        t.tool_events()
    );
    let t = go(
        true,
        vec![Ok(todo(serde_json::json!([]))), Ok(said("ok\n@done"))],
    );
    let plans = t
        .events
        .iter()
        .filter(|e| matches!(e, PluginEvent::Todo { .. }))
        .count();
    assert_eq!(plans, 0, "{:?}", t.events);
}

/// Every model call of the turn is one exchange, billed with the usage it
/// reported, and the tool result's size joins the estimate.
#[test]
fn every_call_of_the_turn_is_in_the_stats() {
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            Ok(said("done\n@done")),
        ],
    );
    assert_eq!(t.stats.exchanges, 2);
    assert_eq!(t.stats.tok_in, 200);
    assert_eq!(t.stats.tok_out, 20);
    let done = t.hops.iter().find(|h| h.kind == HopKind::Done).unwrap();
    assert_eq!(
        done.usage.input_tokens, 100,
        "the latest call's context fill"
    );
}

/// One round on the budget: the second batch still runs, and a last call
/// with no tools on it turns both into the answer.
#[test]
fn past_the_budget_the_batch_runs_and_the_last_word_answers() {
    for native in [true, false] {
        let ask = |p: &str| match native {
            true => calling("fs__read", serde_json::json!({ "path": p })),
            false => said(&format!("@tool fs:read {{\"path\": \"{p}\"}}")),
        };
        let replies = vec![
            Ok(ask("a.rs")),
            Ok(ask("b.rs")),
            Ok(said("both read\n@done")),
        ];
        let t = hop(native, replies, 1, Arc::default());
        assert_eq!(t.ran.len(), 2, "native {native}: {:?}", t.ran);
        let last = t.seen.last().unwrap();
        assert!(
            last.tools.is_empty() && last.turns.is_empty(),
            "native {native}"
        );
        assert!(
            last.prompt.contains("b.rs"),
            "native {native}: {}",
            last.prompt
        );
        let done = t.done().unwrap();
        assert!(done.starts_with("both read"), "native {native}: {done}");
        assert!(
            done.contains("tool budget spent"),
            "native {native}: {done}"
        );
    }
}

/// A model that fails before any tool ran ends the hop on an error card,
/// as a dial does; one that fails after keeps the turn, said under it.
#[test]
fn a_failure_is_an_error_card_and_after_a_tool_the_answer_says_so() {
    let down = || Err(ProviderError::Http("reset".into()));
    let t = go(true, vec![down()]);
    assert_eq!(t.errors(), vec!["http error: reset"]);
    assert_eq!(t.done(), None);
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            down(),
        ],
    );
    assert_eq!(t.errors(), vec!["http error: reset"]);
    let done = t.done().unwrap();
    assert!(
        done.contains("the model's next reply failed: http error: reset"),
        "{done}"
    );
}

/// A stop seen between rounds dials nothing more, and the answer says how far
/// the turn got.
#[test]
fn a_stop_between_rounds_dials_nothing_more() {
    let stop = Arc::new(AtomicBool::new(false));
    struct Stopping(Arc<AtomicBool>, Arc<Mutex<u32>>);
    impl Provider for Stopping {
        fn supports_tools(&self) -> bool {
            true
        }
        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Reply> + Send>> {
            *self.1.lock().unwrap() += 1;
            self.0.store(true, Ordering::Relaxed);
            let c = calling("fs__read", serde_json::json!({"path": "a.rs"}));
            Box::pin(async move { Ok(c) })
        }
    }
    let calls: Arc<Mutex<u32>> = Arc::default();
    let model = Stopping(Arc::clone(&stop), Arc::clone(&calls));
    let agent = ApiAdapter::new("coder", "m", "", None, Arc::new(model)).unwrap();
    let ran: Arc<Mutex<Vec<String>>> = Arc::default();
    let broker = Broker::new(
        Registry::new(vec![Box::new(agent) as Box<dyn Adapter>]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Files(Arc::clone(&ran))))
    .with_cancel_flag(stop);
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut hops = Vec::new();
    broker.run("user", "coder", "read a.rs", "t1", &emit, &mut |h| {
        hops.push(h)
    });
    assert_eq!(*calls.lock().unwrap(), 1, "no dial after the stop");
    assert!(
        ran.lock().unwrap().is_empty(),
        "nothing started after the stop"
    );
}

/// A CLI agent has no model the harness can drive: its hop is still a dial,
/// and its `@tool` lines are the relay's own loop's.
#[test]
fn an_agent_without_an_engine_is_dialed_as_before() {
    struct Cli;
    impl Adapter for Cli {
        fn name(&self) -> &str {
            "coder"
        }
        fn probe(&self) -> bool {
            true
        }
        fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
            Ok(match body.contains("TOOL EXCHANGES THIS TURN") {
                true => "read it\n@done".into(),
                false => "@tool fs:read {\"path\": \"a.rs\"}".into(),
            })
        }
    }
    let ran: Arc<Mutex<Vec<String>>> = Arc::default();
    let broker = Broker::new(
        Registry::new(vec![Box::new(Cli)]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Files(Arc::clone(&ran))));
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut hops = Vec::new();
    broker.run("user", "coder", "read a.rs", "t1", &emit, &mut |h| {
        hops.push(h)
    });
    assert_eq!(ran.lock().unwrap().len(), 1);
    let done = hops.iter().find(|h| h.kind == HopKind::Done).unwrap();
    assert_eq!(done.text, "read it");
}
