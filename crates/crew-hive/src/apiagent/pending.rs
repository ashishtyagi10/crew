//! One reply's tool calls on their way through the blocking pool, for both of
//! the swarm's loops: all at once when every call only reads, in the order
//! written otherwise.
//!
//! A model that knows it needs three files asks for all three in one reply,
//! and each call used to wait for the one before it, so three 300 ms reads
//! took 900 ms where 300 would do. Reads cannot see each other's effects, so
//! they run together; anything else runs in turn, because an edit and the
//! read after it mean something only in that order.
//!
//! OFF THE RUNTIME THREAD either way. `Tools::call` is blocking (an MCP round
//! trip, or a shell command with a two-minute deadline) and the scheduler runs
//! its agents on ONE current-thread runtime beside the bus drain. Awaiting it
//! inline would freeze every other agent in the swarm and stop events reaching
//! the pane, so the whole run would look hung for as long as one tool took.

use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::agent::AgentContext;
use crate::bus::HiveEvent;
use crate::tools::{seen::Seen, ToolCall, Tools};

/// A call's output, and how long it took in milliseconds.
type Called = (Result<String, String>, u64);

/// A call between its `ToolCall` event and its `ToolResult`.
enum Pending {
    /// A repeat, answered by pointing at the round that made it first.
    Pointer(u32),
    /// Running on the blocking pool.
    Running(JoinHandle<Called>),
}

/// How one loop numbers its calls and spells their failures. The native loop
/// notes every call of a round under the round; the text loop notes each under
/// its own exchange, and points only while that exchange is shown whole.
pub(super) struct Rules<'a> {
    /// The number call `k` of the batch is noted in [`Seen`] under.
    pub(super) entry: &'a (dyn Fn(usize) -> u32 + Sync),
    /// The round a repeat of a call points at, or `None` to run it.
    pub(super) repeat: &'a (dyn Fn(&Seen, &ToolCall) -> Option<u32> + Sync),
    /// How a tool's `Err` reads: the native loop flags it on the wire, the
    /// text loop has only the words.
    pub(super) error: fn(String) -> String,
}

/// Run `calls` and answer each, `(ok, text)`, in the order written. Each
/// publishes its own `ToolCall` as it starts and `ToolResult` as it ends.
pub(super) async fn run(
    ctx: &AgentContext,
    tools: &Arc<dyn Tools>,
    seen: &mut Seen,
    calls: &[ToolCall],
    rules: Rules<'_>,
) -> Vec<(bool, String)> {
    let together = calls.len() > 1 && calls.iter().all(|c| tools.repeatable(&c.server, &c.tool));
    // The reads this batch has started: a second copy of one in the same
    // reply points at the first rather than running beside it.
    let mut planned = together.then(|| seen.clone());
    let mut out = Vec::with_capacity(calls.len());
    let mut waiting = Vec::new();
    for (k, call) in calls.iter().enumerate() {
        ctx.bus.publish(HiveEvent::ToolCall {
            agent: ctx.agent.clone(),
            label: call.label(),
            args: call.args.clone(),
        });
        let first = match planned.as_mut() {
            Some(p) => {
                let first = (rules.repeat)(p, call);
                if first.is_none() {
                    p.ran(tools.as_ref(), call, (rules.entry)(k), true);
                }
                first
            }
            None => (rules.repeat)(seen, call),
        };
        let pending = match first {
            Some(round) => Pending::Pointer(round),
            None => start(tools, call),
        };
        match together {
            true => waiting.push(pending),
            false => {
                out.push(finish(ctx, tools, seen, call, (rules.entry)(k), &rules, pending).await)
            }
        }
    }
    for (k, pending) in waiting.into_iter().enumerate() {
        let call = &calls[k];
        out.push(finish(ctx, tools, seen, call, (rules.entry)(k), &rules, pending).await);
    }
    out
}

/// Start `call`, timed where it runs, so a read that waited on its neighbours
/// is not billed for their wait.
fn start(tools: &Arc<dyn Tools>, call: &ToolCall) -> Pending {
    let runner = Arc::clone(tools);
    let c = call.clone();
    Pending::Running(tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let called = runner.call(&c.server, &c.tool, &c.args);
        (called, started.elapsed().as_millis() as u64)
    }))
}

/// Wait for `pending`, note it in `seen` as `entry`, and publish its result.
async fn finish(
    ctx: &AgentContext,
    tools: &Arc<dyn Tools>,
    seen: &mut Seen,
    call: &ToolCall,
    entry: u32,
    rules: &Rules<'_>,
    pending: Pending,
) -> (bool, String) {
    let (ok, text, ms) = match pending {
        Pending::Pointer(first) => (true, Seen::pointer(first), 0),
        Pending::Running(task) => {
            let (called, ms) = task
                .await
                .unwrap_or_else(|e| (Err(format!("tool task failed: {e}")), 0));
            let (ok, text) = match called {
                Ok(v) if v.trim().is_empty() => (true, "(empty result)".to_string()),
                Ok(v) => (!tools.failed(&call.server, &call.tool, &v), v),
                // A refused or failed tool is shown to the agent, not raised
                // as a task failure: "that server is down, use the other one"
                // is a decision the agent can make and this code cannot.
                Err(e) => (false, (rules.error)(e)),
            };
            seen.ran(tools.as_ref(), call, entry, ok);
            (ok, text, ms)
        }
    };
    ctx.bus.publish(HiveEvent::ToolResult {
        agent: ctx.agent.clone(),
        label: call.label(),
        ok,
        text: text.clone(),
        ms,
    });
    (ok, text)
}
