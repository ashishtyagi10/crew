# crew-hive Remote Spill + Sidecar Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Let the scheduler dispatch tasks to out-of-process workers (remote spill) and to external orchestration engines (LangGraph / custom Python) over a JSON wire protocol — built as a testable in-process core so no subprocess is needed to verify it.

**Architecture:** Three additions to `crew-hive`. `wire` defines the serde messages (`RemoteTask` out, `RemoteReply` back) and a `Transport` trait (object-safe, boxed-future) that abstracts "send a task, get a reply" — stdio-to-subprocess in production, in-process loopback in tests. `worker` provides `serve_stdio(reader, writer, handler)` — the worker side that reads `RemoteTask` JSON lines, runs a handler, and writes `RemoteReply` JSON lines (testable with in-memory buffers; an external engine implements the same line protocol to act as a sidecar). `remoteagent` provides `RemoteAgent` — an `Agent` that serializes its task, dispatches over a `Transport`, maps the reply to a `TaskResult`, and emits events — so the existing scheduler runs a fleet of remote agents identically to local ones.

**Tech Stack:** Rust, `serde`/`serde_json`, `tokio`, `std::io::{BufRead, Write}`, `cargo test` + `tokio::test`. No new deps.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total.
- **No new dependencies.**
- The wire protocol is newline-delimited JSON (one message per line) — the same shape the existing plugin host uses, so external engines can speak it easily.
- Boundary types derive `serde::{Serialize, Deserialize}`.
- crew-hive depends on no other crew crate.
- Dead code removed, not suppressed; `#[cfg(test)]` gating allowed.
- Consumes: `crate::board::TaskResult`, `crate::agent::{Agent, AgentContext}`, `crate::bus::{HiveEvent}`, `crate::graph::TaskId`, `crate::bus::AgentId`.

---

### Task 1: Wire protocol + Transport trait

**Files:**
- Create: `crates/crew-hive/src/wire/mod.rs`
- Create: `crates/crew-hive/src/wire/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod wire;` + re-exports)

**Interfaces:**
- Produces (in `crate::wire`):
  - `pub struct RemoteTask { pub agent: u64, pub task: u64, pub prompt: String, pub model: String, pub deps: Vec<DepResult> }` — `Clone, Debug, PartialEq, Serialize, Deserialize`.
  - `pub struct DepResult { pub task: u64, pub output: String, pub success: bool }` — `Clone, Debug, PartialEq, Serialize, Deserialize`. (A wire-friendly copy of a dependency's result.)
  - `pub struct RemoteReply { pub task: u64, pub output: String, pub success: bool, pub input_tokens: u32, pub output_tokens: u32 }` — `Clone, Debug, PartialEq, Serialize, Deserialize`.
  - `pub enum TransportError { Send(String), Recv(String), Decode(String) }` — `Debug`, `Display`, `Error`.
  - `pub trait Transport: Send + Sync { fn dispatch(&self, task: RemoteTask) -> Pin<Box<dyn Future<Output = Result<RemoteReply, TransportError>> + Send>>; }`

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/wire/tests.rs`:
```rust
use super::*;

#[test]
fn remote_task_serde_roundtrip() {
    let t = RemoteTask {
        agent: 1,
        task: 7,
        prompt: "do".into(),
        model: "claude-haiku-4-5".into(),
        deps: vec![DepResult { task: 0, output: "ctx".into(), success: true }],
    };
    let line = serde_json::to_string(&t).unwrap();
    assert!(!line.contains('\n')); // single line for the wire
    assert_eq!(serde_json::from_str::<RemoteTask>(&line).unwrap(), t);
}

#[test]
fn remote_reply_serde_roundtrip() {
    let r = RemoteReply { task: 7, output: "ok".into(), success: true, input_tokens: 3, output_tokens: 1 };
    assert_eq!(serde_json::from_str::<RemoteReply>(&serde_json::to_string(&r).unwrap()).unwrap(), r);
}

#[test]
fn transport_is_object_safe() {
    fn _assert(_: &dyn Transport) {}
}
```

- [ ] **Step 2: Run fail → implement → pass**

Run `cargo test -p crew-hive wire::` (FAIL), implement `wire/mod.rs` (the structs + `TransportError` Display/Error + the `Transport` trait with `use std::future::Future; use std::pin::Pin;`), add `pub mod wire;` + re-exports (`RemoteTask, RemoteReply, DepResult, Transport, TransportError`) to lib.rs, run again (PASS). Keep ≤ 200 lines.

- [ ] **Step 3: Lint + commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets`.
```bash
git add crates/crew-hive/src/wire crates/crew-hive/src/lib.rs
git commit -m "feat(hive): remote wire protocol (RemoteTask/Reply) + Transport trait"
```

---

### Task 2: Loopback transport + stdio worker codec

**Files:**
- Create: `crates/crew-hive/src/worker/mod.rs`
- Create: `crates/crew-hive/src/worker/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod worker;` + re-exports)

**Interfaces:**
- Consumes: `crate::wire::{RemoteTask, RemoteReply, Transport, TransportError}`.
- Produces:
  - `pub struct LoopbackTransport<F> { pub handler: F }` where `F: Fn(RemoteTask) -> RemoteReply + Send + Sync` — a `Transport` impl that runs the handler in-process (for tests and same-process workers). Its `dispatch` clones the result of `handler(task)` into an `Ok`.
  - `pub fn serve_stdio<R: std::io::BufRead, W: std::io::Write>(reader: R, writer: W, handler: impl Fn(RemoteTask) -> RemoteReply) -> std::io::Result<()>` — reads `RemoteTask` JSON, one per line, calls `handler`, writes the `RemoteReply` JSON + `\n`, flushes, until EOF. Lines that fail to parse are skipped (logged to stderr via `eprintln!`). This is the worker side; an external engine implements the same loop in its own language.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/worker/tests.rs`:
```rust
use super::*;
use crate::wire::{DepResult, RemoteReply, RemoteTask, Transport};

fn echo_handler(t: RemoteTask) -> RemoteReply {
    RemoteReply { task: t.task, output: format!("ran:{}", t.task), success: true, input_tokens: 1, output_tokens: 1 }
}

#[tokio::test]
async fn loopback_transport_dispatches_to_handler() {
    let tr = LoopbackTransport { handler: echo_handler };
    let reply = tr
        .dispatch(RemoteTask { agent: 0, task: 9, prompt: "p".into(), model: "m".into(), deps: vec![] })
        .await
        .unwrap();
    assert_eq!(reply.output, "ran:9");
    assert!(reply.success);
}

#[test]
fn serve_stdio_processes_lines() {
    let task = RemoteTask { agent: 0, task: 3, prompt: "p".into(), model: "m".into(), deps: vec![DepResult { task: 1, output: "x".into(), success: true }] };
    let input = format!("{}\n{}\n", serde_json::to_string(&task).unwrap(), "garbage-not-json");
    let mut output = Vec::new();
    serve_stdio(std::io::Cursor::new(input.into_bytes()), &mut output, echo_handler).unwrap();
    let out = String::from_utf8(output).unwrap();
    // exactly one reply line (garbage line skipped)
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1);
    let reply: RemoteReply = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(reply.task, 3);
}
```

- [ ] **Step 2: Run fail → implement → pass**

Run `cargo test -p crew-hive worker::` (FAIL), implement `worker/mod.rs` (the `LoopbackTransport` Transport impl + `serve_stdio` reading `reader.lines()`, parsing each, `writeln!` the reply JSON, `flush`), add `pub mod worker;` + re-exports (`LoopbackTransport`, `serve_stdio`). Run again (PASS). Keep ≤ 200 lines.

- [ ] **Step 3: Lint + commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets`.
```bash
git add crates/crew-hive/src/worker crates/crew-hive/src/lib.rs
git commit -m "feat(hive): loopback transport + stdio worker codec (sidecar protocol)"
```

---

### Task 3: RemoteAgent + scheduler integration

**Files:**
- Create: `crates/crew-hive/src/remoteagent/mod.rs`
- Create: `crates/crew-hive/src/remoteagent/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod remoteagent;` + re-export `RemoteAgent`)
- Modify: `crates/crew-hive/tests/engine.rs` (add a scheduler-over-remote integration test)

**Interfaces:**
- Consumes: `crate::agent::{Agent, AgentContext}`, `crate::wire::{Transport, RemoteTask, DepResult}`, `crate::board::TaskResult`, `crate::bus::HiveEvent`.
- Produces:
  - `pub struct RemoteAgent { transport: std::sync::Arc<dyn Transport> }` with `pub fn new(transport: Arc<dyn Transport>) -> Self`.
  - `Agent` impl: builds a `RemoteTask` from `ctx` (agent=ctx.agent.0, task=ctx.task.id.0, prompt=ctx.task.prompt, model=ctx.task.model.model_id(), deps mapped from `ctx.deps` to `DepResult`), calls `transport.dispatch`, emits `HiveEvent::{TokenDelta, OutputChunk}` from the reply, and returns `TaskResult { task: ctx.task.id, output: reply.output, success: reply.success }`. On `TransportError`, emits `HiveEvent::Failed` and returns `success: false`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/remoteagent/tests.rs`:
```rust
use super::*;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::wire::{LoopbackTransportHelper, RemoteReply}; // see note below
use std::sync::Arc;

fn spec(id: u64) -> TaskSpec {
    TaskSpec { id: TaskId(id), title: "t".into(), agent: AgentKind::Api { system: None }, model: ModelTier::Standard, deps: vec![], prompt: "p".into() }
}

#[tokio::test]
async fn remote_agent_dispatches_and_returns_result() {
    // Loopback transport whose handler succeeds.
    let tr = crate::worker::LoopbackTransport {
        handler: |t: crate::wire::RemoteTask| RemoteReply { task: t.task, output: "remote-ok".into(), success: true, input_tokens: 2, output_tokens: 2 },
    };
    let agent = RemoteAgent::new(Arc::new(tr));
    let bus = EventBus::new(32);
    let ctx = AgentContext { agent: AgentId(0), task: spec(5), deps: vec![], bus };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.output, "remote-ok");
    assert_eq!(result.task, TaskId(5));
}
```
(Note: drop the bogus `LoopbackTransportHelper` import — use `crate::worker::LoopbackTransport` directly as shown in the body. The implementer should write the imports to match what's actually used.)

- [ ] **Step 2: Run fail → implement → pass**

Run `cargo test -p crew-hive remoteagent::` (FAIL), implement `remoteagent/mod.rs`, add `pub mod remoteagent;` + re-export. Run again (PASS). Keep ≤ 200 lines.

- [ ] **Step 3: Scheduler-over-remote integration test**

Append to `crates/crew-hive/tests/engine.rs` a test that runs a small graph through `Scheduler` with an `AgentFactory` that makes `RemoteAgent`s over a shared `LoopbackTransport` (handler returns success), asserting all tasks `done` + results in the board. This proves the scheduler treats remote agents identically to local ones (the remote-spill path).

```rust
#[tokio::test]
async fn scheduler_runs_remote_agents() {
    use crew_hive::{Agent, AgentContext, AgentFactory, AgentKind, Blackboard, EventBus, RemoteAgent,
        Scheduler, StubPlanner, Planner};
    use crew_hive::wire::{RemoteReply, RemoteTask};
    use crew_hive::worker::LoopbackTransport;
    use std::sync::Arc;

    // A Transport-making factory. LoopbackTransport's handler is a fn pointer (Copy),
    // so each agent can hold its own Arc<dyn Transport>.
    struct RemoteFactory;
    impl AgentFactory for RemoteFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            let tr = LoopbackTransport {
                handler: |t: RemoteTask| RemoteReply { task: t.task, output: "ok".into(), success: true, input_tokens: 1, output_tokens: 1 },
            };
            Box::new(RemoteAgent::new(Arc::new(tr)))
        }
    }

    let graph = StubPlanner { fanout: 3 }.plan("g").await.unwrap();
    let n = graph.len();
    let board = Blackboard::new();
    let out = Scheduler::new(graph, board.clone(), EventBus::new(128), Arc::new(RemoteFactory), 8).run().await;
    assert_eq!(out.done.len(), n);
    assert_eq!(board.result_count().await, n);
}
```
(If `crew_hive::wire` / `crew_hive::worker` aren't public module paths, re-export the needed items at the crate root instead and adjust imports. Ensure `RemoteAgent`, `Agent`, `AgentContext`, `AgentFactory` are all re-exported.)

- [ ] **Step 4: Full gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets`.
```bash
git add crates/crew-hive/src/remoteagent crates/crew-hive/src/lib.rs crates/crew-hive/tests/engine.rs
git commit -m "feat(hive): RemoteAgent + scheduler-over-remote integration test"
```

---

## Self-Review

- **Spec coverage:** "Sidecar bridge: external engines run as sidecar processes speaking a protocol (JSON-RPC over stdio); crew renders/controls them like native agents" → `wire` protocol + `serve_stdio` (the line both a Rust worker and an external engine implement) + `RemoteAgent` (the scheduler's handle to a remote worker). "Remote spill: scheduler dispatches nodes to remote workers" → `RemoteAgent` over a `Transport`, proven via the scheduler integration test. The real subprocess `StdioTransport` (spawn child, pipe stdin/stdout) is a thin production layer over the tested `serve_stdio` codec + `Transport` trait — wiring it to `tokio::process` is a follow-on that needs a worker binary to verify end-to-end. ✅
- **Placeholder scan:** complete interfaces + tests; the one bogus import in the Task 3 unit test is explicitly flagged for the implementer to drop. ✅
- **No new deps / no GUI / no LLM (loopback path).** ✅
- **Object safety:** `Transport` and `RemoteAgent` use boxed futures → `Arc<dyn Transport>` / `Box<dyn Agent>` valid. ✅

## Where this sits

The last engine plan. With it, crew-hive can run agents locally, as native API calls, OR dispatched over a wire to remote workers / external engines — the full "Hive" substrate from the swarm goal. Remaining work to reach the user-visible product is the **GPU swarm-view rendering + engine→terminal wiring in crew-app**, which needs a GPU to runtime-verify and is therefore deferred to hands-on testing.
