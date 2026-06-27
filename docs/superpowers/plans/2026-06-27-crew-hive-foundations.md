# crew-hive Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Create a new `crew-hive` crate holding the orchestration substrate's foundations — a typed task-graph (DAG), a non-blocking event bus, and a fleet telemetry model — all pure/async-channel logic, fully unit-tested, with no LLM and no GUI dependencies.

**Architecture:** `crew-hive` is a library crate. Three independent modules: `graph` (task DAG: specs, dependencies, readiness, cycle/validation), `bus` (a `tokio::sync::broadcast`-based event channel so renderers/telemetry subscribe without blocking workers), `telemetry` (per-agent state + a fleet snapshot aggregated by applying events). This is the data + eventing backbone the scheduler (next plan) and swarm view (later) build on. Purely additive — no existing crate changes except adding `crew-hive` to the workspace members.

**Tech Stack:** Rust, `tokio` (sync::broadcast), `serde`/`serde_json` (graph + events are serializable for the future sidecar bridge), `cargo test` (+ `tokio::test` for async). All are existing `[workspace.dependencies]`.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total (imports, whitespace, doc comments included). Split into submodules before crossing it.
- **No new dependencies** — use only existing `[workspace.dependencies]` (`tokio`, `tokio-stream`, `futures`, `serde`, `serde_json`).
- Every public type that crosses the eventual sidecar/remote boundary derives `serde::{Serialize, Deserialize}` (graph specs, events, telemetry).
- Dead code is removed, not suppressed (no `#[allow(dead_code)]`); `#[cfg(test)]`-gating test-only items is allowed.
- IDs are newtypes (`TaskId(u64)`, `AgentId(u64)`) — never bare integers in public APIs.
- This crate must NOT depend on `crew-app`, `crew-render`, or `crew-term` (it is the dependency-free core).

---

### Task 1: Crate scaffold + task-graph

Create the crate and the task DAG: task specs, dependency readiness, and validation (deps exist, no cycles).

**Files:**
- Create: `crates/crew-hive/Cargo.toml`
- Create: `crates/crew-hive/src/lib.rs`
- Create: `crates/crew-hive/src/graph/mod.rs`
- Create: `crates/crew-hive/src/graph/spec.rs`
- Create: `crates/crew-hive/src/graph/tests.rs`
- Modify: `Cargo.toml` (add `crates/crew-hive` to `[workspace] members`)

**Interfaces:**
- Produces (in `crate::graph`):
  - `pub struct TaskId(pub u64);` — `Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Serialize, Deserialize`.
  - `pub enum AgentKind { Pty { command: String, args: Vec<String> }, Api { system: Option<String> } }` — `Clone, Debug, Serialize, Deserialize, PartialEq`. (Pty = a CLI agent in a terminal; Api = a headless LLM-call agent. More variants later.)
  - `pub enum ModelTier { Cheap, Standard, Capable }` — `Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq`.
  - `pub enum TaskState { Pending, Ready, Running, Done, Failed, Cancelled }` — `Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq`.
  - `pub struct TaskSpec { pub id: TaskId, pub title: String, pub agent: AgentKind, pub model: ModelTier, pub deps: Vec<TaskId>, pub prompt: String }` — `Clone, Debug, Serialize, Deserialize`.
  - `pub struct TaskGraph { tasks: Vec<TaskSpec> }` with:
    - `pub fn new(tasks: Vec<TaskSpec>) -> Result<Self, GraphError>` — validates (unique ids; every dep refers to an existing id; no cycles) and returns `Err` otherwise.
    - `pub fn tasks(&self) -> &[TaskSpec]`
    - `pub fn get(&self, id: TaskId) -> Option<&TaskSpec>`
    - `pub fn ready(&self, done: &std::collections::HashSet<TaskId>) -> Vec<TaskId>` — ids whose deps are all in `done` and that are not themselves in `done`, in ascending id order (deterministic).
    - `pub fn len(&self) -> usize`, `pub fn is_empty(&self) -> bool`.
  - `pub enum GraphError { DuplicateId(TaskId), MissingDep { task: TaskId, dep: TaskId }, Cycle }` — `Debug, PartialEq`, plus `impl std::fmt::Display` and `impl std::error::Error`.

- [ ] **Step 1: Create the crate manifest**

Create `crates/crew-hive/Cargo.toml`:

```toml
[package]
name = "crew-hive"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
```

Add `"crates/crew-hive",` to the `members` array in the root `Cargo.toml` (alongside the other crates).

- [ ] **Step 2: Create `lib.rs`**

Create `crates/crew-hive/src/lib.rs`:

```rust
//! crew-hive: the orchestration substrate for running many agents toward a
//! task. Foundations: a typed task-graph (`graph`), a non-blocking event bus
//! (`bus`), and a fleet telemetry model (`telemetry`).

pub mod graph;
```

(`bus` and `telemetry` modules are added in Tasks 2 and 3.)

- [ ] **Step 3: Write the failing tests**

Create `crates/crew-hive/src/graph/tests.rs`:

```rust
use super::*;
use std::collections::HashSet;

fn spec(id: u64, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: format!("t{id}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: String::new(),
    }
}

#[test]
fn new_accepts_valid_dag() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[0, 1])]).unwrap();
    assert_eq!(g.len(), 3);
    assert!(!g.is_empty());
}

#[test]
fn new_rejects_duplicate_id() {
    let err = TaskGraph::new(vec![spec(0, &[]), spec(0, &[])]).unwrap_err();
    assert_eq!(err, GraphError::DuplicateId(TaskId(0)));
}

#[test]
fn new_rejects_missing_dep() {
    let err = TaskGraph::new(vec![spec(0, &[7])]).unwrap_err();
    assert_eq!(err, GraphError::MissingDep { task: TaskId(0), dep: TaskId(7) });
}

#[test]
fn new_rejects_cycle() {
    let err = TaskGraph::new(vec![spec(0, &[1]), spec(1, &[0])]).unwrap_err();
    assert_eq!(err, GraphError::Cycle);
}

#[test]
fn ready_returns_roots_first() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[])]).unwrap();
    let done = HashSet::new();
    assert_eq!(g.ready(&done), vec![TaskId(0), TaskId(2)]);
}

#[test]
fn ready_unlocks_dependents_and_skips_done() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[0, 1])]).unwrap();
    let mut done = HashSet::new();
    done.insert(TaskId(0));
    assert_eq!(g.ready(&done), vec![TaskId(1)]); // 2 still blocked on 1; 0 is done
    done.insert(TaskId(1));
    assert_eq!(g.ready(&done), vec![TaskId(2)]);
    done.insert(TaskId(2));
    assert!(g.ready(&done).is_empty());
}

#[test]
fn get_and_serde_roundtrip() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0])]).unwrap();
    assert_eq!(g.get(TaskId(1)).unwrap().title, "t1");
    assert!(g.get(TaskId(9)).is_none());
    let json = serde_json::to_string(g.tasks()).unwrap();
    let back: Vec<TaskSpec> = serde_json::from_str(&json).unwrap();
    assert_eq!(back.len(), 2);
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p crew-hive`
Expected: FAIL — compile error, `graph` items not defined.

- [ ] **Step 5: Implement the graph types**

Create `crates/crew-hive/src/graph/spec.rs` with `TaskId`, `AgentKind`, `ModelTier`, `TaskState`, `TaskSpec`, and `GraphError` (the data types + derives listed in Interfaces; `GraphError` gets `Display` + `Error`). Keep this file types-only.

Create `crates/crew-hive/src/graph/mod.rs` with the `TaskGraph` struct and its impl:
- `new`: build a `HashSet` of ids checking duplicates (`DuplicateId`); verify each dep exists (`MissingDep`); run a DFS/Kahn cycle check (`Cycle`).
- `ready`: iterate tasks sorted by id; include those not in `done` whose every dep is in `done`.
- Re-export the spec types and declare `#[cfg(test)] mod tests;`.

```rust
//! Task DAG: specs, dependency readiness, and validation.
mod spec;
#[cfg(test)]
mod tests;

pub use spec::{AgentKind, GraphError, ModelTier, TaskId, TaskSpec, TaskState};

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct TaskGraph {
    tasks: Vec<TaskSpec>,
}

impl TaskGraph {
    pub fn new(tasks: Vec<TaskSpec>) -> Result<Self, GraphError> {
        let mut ids = HashSet::new();
        for t in &tasks {
            if !ids.insert(t.id) {
                return Err(GraphError::DuplicateId(t.id));
            }
        }
        for t in &tasks {
            for d in &t.deps {
                if !ids.contains(d) {
                    return Err(GraphError::MissingDep { task: t.id, dep: *d });
                }
            }
        }
        let g = Self { tasks };
        if g.has_cycle() {
            return Err(GraphError::Cycle);
        }
        Ok(g)
    }

    pub fn tasks(&self) -> &[TaskSpec] {
        &self.tasks
    }

    pub fn get(&self, id: TaskId) -> Option<&TaskSpec> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn ready(&self, done: &HashSet<TaskId>) -> Vec<TaskId> {
        let mut out: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|t| !done.contains(&t.id) && t.deps.iter().all(|d| done.contains(d)))
            .map(|t| t.id)
            .collect();
        out.sort_unstable();
        out
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Kahn's algorithm: if we cannot remove all nodes by repeatedly removing
    /// those with no unsatisfied deps, a cycle exists.
    fn has_cycle(&self) -> bool {
        let mut done: HashSet<TaskId> = HashSet::new();
        loop {
            let next = self.ready(&done);
            if next.is_empty() {
                return done.len() != self.tasks.len();
            }
            for id in next {
                done.insert(id);
            }
        }
    }
}
```

Write `spec.rs` to satisfy the above (full type defs with derives + `GraphError` Display/Error). Keep each file ≤ 200 lines.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p crew-hive`
Expected: PASS — all graph tests green.

- [ ] **Step 7: Format, lint, commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets` (warning-free).

```bash
git add crates/crew-hive/Cargo.toml crates/crew-hive/src/lib.rs crates/crew-hive/src/graph Cargo.toml Cargo.lock
git commit -m "feat(hive): crew-hive crate + task-graph (DAG, readiness, validation)"
```

---

### Task 2: Event bus

A non-blocking publish/subscribe bus over `tokio::sync::broadcast`, carrying typed orchestration events. Workers publish; renderers/telemetry subscribe without blocking the workers.

**Files:**
- Create: `crates/crew-hive/src/bus/mod.rs`
- Create: `crates/crew-hive/src/bus/event.rs`
- Create: `crates/crew-hive/src/bus/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod bus;`)

**Interfaces:**
- Consumes: `crate::graph::{TaskId, TaskState}`.
- Produces (in `crate::bus`):
  - `pub struct AgentId(pub u64);` — same derives as `TaskId`.
  - `pub enum HiveEvent { TaskStateChanged { task: TaskId, state: TaskState }, AgentSpawned { agent: AgentId, task: TaskId }, TokenDelta { agent: AgentId, input: u32, output: u32 }, CostDelta { agent: AgentId, micros_usd: u64 }, OutputChunk { agent: AgentId, text: String }, Failed { agent: AgentId, error: String } }` — `Clone, Debug, Serialize, Deserialize, PartialEq`.
  - `pub struct EventBus { tx: tokio::sync::broadcast::Sender<HiveEvent> }` with:
    - `pub fn new(capacity: usize) -> Self`
    - `pub fn publish(&self, ev: HiveEvent)` — best-effort; a send error when there are no subscribers is ignored (workers must not block/fail on an absent UI).
    - `pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<HiveEvent>`
    - `Clone` (cloning shares the same channel).

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/bus/tests.rs`:

```rust
use super::*;
use crate::graph::{TaskId, TaskState};

#[tokio::test]
async fn subscriber_receives_published_events() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();
    bus.publish(HiveEvent::TaskStateChanged { task: TaskId(1), state: TaskState::Running });
    let ev = rx.recv().await.unwrap();
    assert_eq!(ev, HiveEvent::TaskStateChanged { task: TaskId(1), state: TaskState::Running });
}

#[test]
fn publish_without_subscribers_does_not_panic() {
    let bus = EventBus::new(8);
    // No subscriber; must be a no-op, not an error/panic.
    bus.publish(HiveEvent::Failed { agent: AgentId(0), error: "x".into() });
}

#[tokio::test]
async fn two_subscribers_both_receive() {
    let bus = EventBus::new(16);
    let mut a = bus.subscribe();
    let mut b = bus.subscribe();
    bus.publish(HiveEvent::TokenDelta { agent: AgentId(3), input: 10, output: 20 });
    let ea = a.recv().await.unwrap();
    let eb = b.recv().await.unwrap();
    assert_eq!(ea, eb);
}

#[test]
fn agentid_event_serde_roundtrip() {
    let ev = HiveEvent::CostDelta { agent: AgentId(2), micros_usd: 1500 };
    let json = serde_json::to_string(&ev).unwrap();
    let back: HiveEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(ev, back);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-hive bus::`
Expected: FAIL — `bus` items not defined.

- [ ] **Step 3: Implement the bus**

Create `crates/crew-hive/src/bus/event.rs` with `AgentId` and `HiveEvent` (the enum + derives above).

Create `crates/crew-hive/src/bus/mod.rs`:

```rust
//! Non-blocking event bus: workers publish `HiveEvent`s; UI/telemetry
//! subscribe. Backed by `tokio::sync::broadcast` so a slow/absent subscriber
//! never blocks a worker.
mod event;
#[cfg(test)]
mod tests;

pub use event::{AgentId, HiveEvent};

use tokio::sync::broadcast;

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<HiveEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Best-effort publish. With no subscribers `send` returns `Err`; that is
    /// expected (headless runs) and intentionally ignored — never block work.
    pub fn publish(&self, ev: HiveEvent) {
        let _ = self.tx.send(ev);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HiveEvent> {
        self.tx.subscribe()
    }
}
```

Add `pub mod bus;` to `lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-hive bus::`
Expected: PASS.

- [ ] **Step 5: Format, lint, commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets` (warning-free).

```bash
git add crates/crew-hive/src/bus crates/crew-hive/src/lib.rs
git commit -m "feat(hive): non-blocking event bus (broadcast) + HiveEvent"
```

---

### Task 3: Fleet telemetry

A per-agent telemetry record and a fleet snapshot built by applying `HiveEvent`s. This is what the swarm view (later) renders.

**Files:**
- Create: `crates/crew-hive/src/telemetry/mod.rs`
- Create: `crates/crew-hive/src/telemetry/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod telemetry;`)

**Interfaces:**
- Consumes: `crate::bus::{AgentId, HiveEvent}`, `crate::graph::{TaskId, TaskState}`.
- Produces (in `crate::telemetry`):
  - `pub struct AgentTelemetry { pub agent: AgentId, pub task: TaskId, pub state: TaskState, pub tokens_in: u32, pub tokens_out: u32, pub micros_usd: u64, pub last_line: String }` — `Clone, Debug, Serialize, Deserialize, PartialEq`.
  - `pub struct Fleet { agents: std::collections::BTreeMap<u64, AgentTelemetry> }` with:
    - `pub fn new() -> Self`, `Default`.
    - `pub fn apply(&mut self, ev: &HiveEvent)` — updates state: `AgentSpawned` inserts a record (state `Running`); `TokenDelta` adds to in/out; `CostDelta` adds micros; `OutputChunk` sets `last_line` to the chunk's final non-empty line; `Failed` sets state `Failed` and `last_line` to the error; `TaskStateChanged` updates the matching agent's state if an agent for that task exists. Events for unknown agents (except `AgentSpawned`) are ignored.
    - `pub fn agents(&self) -> impl Iterator<Item = &AgentTelemetry>` — ascending by agent id.
    - `pub fn get(&self, agent: AgentId) -> Option<&AgentTelemetry>`
    - `pub fn totals(&self) -> FleetTotals`
  - `pub struct FleetTotals { pub live: usize, pub done: usize, pub failed: usize, pub tokens_in: u64, pub tokens_out: u64, pub micros_usd: u64 }` — `Clone, Copy, Debug, PartialEq, Eq`. (`live` = state Running; `done` = Done; `failed` = Failed.)

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/telemetry/tests.rs`:

```rust
use super::*;
use crate::bus::{AgentId, HiveEvent};
use crate::graph::{TaskId, TaskState};

#[test]
fn spawn_then_tokens_and_cost_accumulate() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned { agent: AgentId(1), task: TaskId(5) });
    f.apply(&HiveEvent::TokenDelta { agent: AgentId(1), input: 10, output: 4 });
    f.apply(&HiveEvent::TokenDelta { agent: AgentId(1), input: 0, output: 6 });
    f.apply(&HiveEvent::CostDelta { agent: AgentId(1), micros_usd: 2500 });
    let a = f.get(AgentId(1)).unwrap();
    assert_eq!((a.tokens_in, a.tokens_out, a.micros_usd), (10, 10, 2500));
    assert_eq!(a.task, TaskId(5));
    assert_eq!(a.state, TaskState::Running);
}

#[test]
fn output_chunk_sets_last_nonempty_line() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned { agent: AgentId(1), task: TaskId(0) });
    f.apply(&HiveEvent::OutputChunk { agent: AgentId(1), text: "building...\nok\n".into() });
    assert_eq!(f.get(AgentId(1)).unwrap().last_line, "ok");
}

#[test]
fn failed_sets_state_and_message() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned { agent: AgentId(2), task: TaskId(1) });
    f.apply(&HiveEvent::Failed { agent: AgentId(2), error: "boom".into() });
    let a = f.get(AgentId(2)).unwrap();
    assert_eq!(a.state, TaskState::Failed);
    assert_eq!(a.last_line, "boom");
}

#[test]
fn unknown_agent_events_ignored() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::TokenDelta { agent: AgentId(9), input: 1, output: 1 });
    assert!(f.get(AgentId(9)).is_none());
}

#[test]
fn task_state_change_updates_matching_agent() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned { agent: AgentId(1), task: TaskId(7) });
    f.apply(&HiveEvent::TaskStateChanged { task: TaskId(7), state: TaskState::Done });
    assert_eq!(f.get(AgentId(1)).unwrap().state, TaskState::Done);
}

#[test]
fn totals_aggregate_across_agents() {
    let mut f = Fleet::new();
    f.apply(&HiveEvent::AgentSpawned { agent: AgentId(1), task: TaskId(0) });
    f.apply(&HiveEvent::AgentSpawned { agent: AgentId(2), task: TaskId(1) });
    f.apply(&HiveEvent::TokenDelta { agent: AgentId(1), input: 5, output: 7 });
    f.apply(&HiveEvent::CostDelta { agent: AgentId(2), micros_usd: 900 });
    f.apply(&HiveEvent::TaskStateChanged { task: TaskId(1), state: TaskState::Done });
    let t = f.totals();
    assert_eq!(t.live, 1);
    assert_eq!(t.done, 1);
    assert_eq!(t.failed, 0);
    assert_eq!((t.tokens_in, t.tokens_out, t.micros_usd), (5, 7, 900));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-hive telemetry::`
Expected: FAIL — telemetry items not defined.

- [ ] **Step 3: Implement telemetry**

Create `crates/crew-hive/src/telemetry/mod.rs` implementing `AgentTelemetry`, `Fleet` (keyed by `agent.0` in a `BTreeMap` for ascending iteration), `apply` (per the rules in Interfaces — for `OutputChunk`, set `last_line` to the last line of `text` that is non-empty after trimming trailing newlines; if all empty, leave unchanged), `totals`, and declare `#[cfg(test)] mod tests;`. Keep ≤ 200 lines (split into a `record.rs` submodule if needed).

Add `pub mod telemetry;` to `lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-hive telemetry::`
Expected: PASS.

- [ ] **Step 5: Full crate gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets` (whole workspace warning-free; crew-hive is additive so nothing else changes).

```bash
git add crates/crew-hive/src/telemetry crates/crew-hive/src/lib.rs
git commit -m "feat(hive): fleet telemetry model (apply events -> snapshot + totals)"
```

---

## Self-Review

- **Spec coverage:** Hive "Scheduler/Bus/Blackboard/Planner" backbone — this plan delivers the **task-graph** (what the scheduler executes), the **bus** (status/token/cost stream feeding the swarm view), and **telemetry** (the observable-by-default requirement). Scheduler, agent pool, blackboard, and planner are the next plans. ✅
- **Placeholder scan:** Task 1 and 2 give complete code; Task 3 gives complete interfaces + test cases + precise apply-rules (the implementer writes the straightforward match). No TODO/TBD. ✅
- **Type consistency:** `TaskId`/`AgentId` newtypes, `HiveEvent`, `TaskState`, `Fleet`/`AgentTelemetry`/`FleetTotals`, `TaskGraph`/`TaskSpec`/`AgentKind`/`ModelTier`/`GraphError` used identically across tasks. `ready(&HashSet<TaskId>)` signature stable between graph impl and scheduler (next plan). ✅
- **No new deps / no GUI / no LLM:** only tokio+serde; pure logic; fully unit-testable headless. ✅
- **File sizes:** every file targeted ≤ 200; split notes included. ✅

## Where this sits

First plan of the orchestration engine (the "Hive") from `docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md`. Next: **blackboard** (shared results), then the **tokio DAG scheduler + agent pool** that consumes `TaskGraph` + `ready()` and emits `HiveEvent`s onto this bus.
```
