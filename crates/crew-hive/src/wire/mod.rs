//! Wire protocol: JSON-line types exchanged between crew and an out-of-process engine.
//!
//! The bridge existed before this and was not worth crossing. `RemoteTask` carried `{agent,
//! task, prompt, model, deps}` and the reply carried a string, so a graph engine behind it could
//! only return text — a slower [`crate::apiagent::ApiAgent`] with none of its tools. The goal
//! (`docs/superpowers/goals/2026-09-01-close-the-open-goals.md`, Pillar 4) is the opposite: an
//! engine that can do things crew's DAG cannot express, using crew's tools and crew's
//! credentials.
//!
//! So the protocol is a CONVERSATION rather than a request and a response:
//!
//! ```text
//! crew  → worker : {"kind":"task", …, "tools":[…], "state":…}
//! worker → crew  : {"kind":"delta","text":"thinking…"}      (zero or more)
//! worker → crew  : {"kind":"call","id":"c1","tool":"sys:run","args":"{…}"}
//! crew  → worker : {"kind":"result","id":"c1","output":"…","ok":true}
//! worker → crew  : {"kind":"done","output":"…","success":true,…,"state":…}
//! ```
//!
//! **A tool call goes to crew, never to the worker's own key.** That is the whole reason the
//! protocol gained a turn: the sidecar names a tool, crew runs it through the same gate and the
//! same ledger as everything else, and the sidecar never holds a credential. An engine that
//! could authenticate on its own would be a second, unaudited way for crew to reach the world.
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;
use std::pin::Pin;

#[cfg(test)]
mod tests;

/// One dependency's result sent alongside a remote task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DepResult {
    pub task: u64,
    pub output: String,
    pub success: bool,
}

/// One tool the worker may ask crew to run, as crew names it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDecl {
    /// `server:tool` — crew's spelling, which is also what a `call` names.
    pub name: String,
    pub description: String,
    /// JSON Schema for the arguments; `{"type":"object"}` when the source declared none.
    pub input_schema: serde_json::Value,
}

/// Dispatch envelope sent to a remote worker.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RemoteTask {
    pub agent: u64,
    pub task: u64,
    pub prompt: String,
    pub model: String,
    pub deps: Vec<DepResult>,
    /// What the worker may ask crew to run on its behalf. Empty when the session has no tools,
    /// which is also what an older worker sees — the field defaults, so a worker written
    /// against the first protocol still parses a task from this one.
    #[serde(default)]
    pub tools: Vec<ToolDecl>,
    /// Whatever the worker handed back last time, returned verbatim. crew never looks inside:
    /// resumability belongs to the engine that has cycles, and a checkpoint crew could read
    /// would be a checkpoint crew would eventually be expected to migrate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<serde_json::Value>,
}

/// Reply envelope received from a remote worker.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RemoteReply {
    pub task: u64,
    pub output: String,
    pub success: bool,
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Opaque state to hand back on the next task for this graph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<serde_json::Value>,
}

/// What crew says to a worker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum HostMsg {
    /// Do this.
    Task(RemoteTask),
    /// What the tool you asked for returned. `ok` false means it refused or failed — which is
    /// information the worker can act on, not a reason to stop.
    Result {
        id: String,
        output: String,
        ok: bool,
    },
}

/// What a worker says to crew.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum WorkerMsg {
    /// A fragment of thinking, streamed as it happens.
    Delta { text: String },
    /// Run this tool for me. `tool` is crew's `server:tool`; `args` is a JSON object as text.
    Call {
        id: String,
        tool: String,
        args: String,
    },
    /// Finished.
    Done(RemoteReply),
}

/// Errors that may arise during transport dispatch.
#[derive(Debug)]
pub enum TransportError {
    Send(String),
    Recv(String),
    Decode(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Send(s) => write!(f, "transport send error: {s}"),
            TransportError::Recv(s) => write!(f, "transport recv error: {s}"),
            TransportError::Decode(s) => write!(f, "transport decode error: {s}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// What a transport needs from crew while a task is running: the tools it may run on the
/// worker's behalf, and somewhere to put streamed text.
///
/// Passed as one value rather than three arguments because every transport needs all of it and
/// none of it belongs to the transport: the tools are the session's, the sink is the bus's.
pub struct Host<'a> {
    pub tools: Option<&'a dyn crate::tools::Tools>,
    /// Called with each streamed fragment. A transport that streams nothing never calls it.
    pub on_delta: &'a (dyn Fn(&str) + Send + Sync),
}

impl Host<'_> {
    /// Run one tool for a worker, or say why not. Never propagates: a tool that refuses is
    /// something the worker can act on.
    pub fn call(&self, tool: &str, args: &str) -> (String, bool) {
        // The NAME is checked first, and deliberately: `run` is malformed whether or not this
        // session has tools, and answering "no tools here" to it sends the worker looking for
        // the wrong problem.
        let Some((server, name)) = tool.split_once(':') else {
            return (format!("{tool:?} is not a server:tool name"), false);
        };
        let Some(tools) = self.tools else {
            return ("this crew session has no tools".into(), false);
        };
        match tools.call(server, name, args) {
            Ok(out) => (out, true),
            Err(e) => (e, false),
        }
    }
}

/// Object-safe transport: runs one `RemoteTask` to completion, servicing whatever the worker
/// asks for on the way. Uses a boxed future so `Arc<dyn Transport>` works without async-trait.
pub trait Transport: Send + Sync {
    /// The future borrows the host, which is what lets a worker call back into crew's tools
    /// mid-task: the host is the caller's (the session's tools, the bus's sink), so it cannot be
    /// moved into a `'static` future.
    fn dispatch<'a>(
        &'a self,
        task: RemoteTask,
        host: Host<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteReply, TransportError>> + Send + 'a>>;
}
