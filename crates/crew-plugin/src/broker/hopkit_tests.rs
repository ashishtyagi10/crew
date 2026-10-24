//! What the hop turn tests share: a scripted model behind a real
//! `ApiAdapter`, a file surface that keeps its calls, and one hop of it
//! driven whole through `Broker::run`.
use std::sync::atomic::AtomicBool;
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

pub(super) type Reply = Result<Completion, ProviderError>;

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

pub(super) fn said(text: &str) -> Completion {
    Completion {
        text: text.into(),
        input_tokens: 100,
        output_tokens: 10,
        ..Default::default()
    }
}

/// A native reply that calls `name` with `input`.
pub(super) fn calling(name: &str, input: serde_json::Value) -> Completion {
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
pub(super) struct Files(pub(super) Arc<Mutex<Vec<String>>>);

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
pub(super) struct Turn {
    pub(super) hops: Vec<Hop>,
    pub(super) events: Vec<PluginEvent>,
    pub(super) seen: Vec<CompletionRequest>,
    pub(super) ran: Vec<String>,
    pub(super) stats: crate::broker::RunStats,
}

impl Turn {
    pub(super) fn done(&self) -> Option<&str> {
        self.hops
            .iter()
            .find(|h| h.kind == HopKind::Done)
            .map(|h| h.text.as_str())
    }
    pub(super) fn errors(&self) -> Vec<&str> {
        self.hops
            .iter()
            .filter(|h| h.kind == HopKind::Error)
            .map(|h| h.text.as_str())
            .collect()
    }
    pub(super) fn tool_events(&self) -> Vec<&HiveEvent> {
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
pub(super) fn hop(native: bool, replies: Vec<Reply>, rounds: u32, stop: Arc<AtomicBool>) -> Turn {
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

pub(super) fn go(native: bool, replies: Vec<Reply>) -> Turn {
    hop(native, replies, 4, Arc::default())
}
