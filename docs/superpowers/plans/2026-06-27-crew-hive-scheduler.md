# crew-hive Scheduler + Agent Pool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add the orchestration core to `crew-hive`: an `Agent` abstraction (object-safe, boxed-future — no new deps), and a `Scheduler` that executes a `TaskGraph` to completion over a bounded `tokio` worker pool — feeding each agent its dependencies' results from the `Blackboard`, recording results back, emitting `HiveEvent`s, and propagating failure/cancellation.

**Architecture:** Two modules. `agent` defines `trait Agent` (`fn run(&self, ctx) -> Pin<Box<dyn Future<Output=TaskResult> + Send>>` — dyn-compatible without the `async-trait` crate), `AgentContext` (owned: agent id, task spec, gathered dep results, bus handle), and `trait AgentFactory` (maps an `AgentKind` to a boxed agent), plus a `StubAgent`/`StubFactory` for headless tests. `sched` defines `Scheduler`: a `tokio::task::JoinSet` of in-flight agents gated by a `tokio::sync::Semaphore` (the concurrency cap), driven by `TaskGraph::ready()`; results land in the `Blackboard`, state transitions emit on the `EventBus`, and a failed/cancelled task cascades cancellation to its dependents.

**Tech Stack:** Rust, `tokio` (task::JoinSet, sync::Semaphore, time for test pacing), `std::future::Future`, `cargo test` + `tokio::test`. All existing deps.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total. Split aggressively (agent/sched submodules).
- **No new dependencies** — NO `async-trait`. Use manual `Pin<Box<dyn Future + Send>>` return types for object safety.
- crew-hive depends on no other crew crate; boundary types stay serde-derivable.
- Dead code removed, not suppressed; `#[cfg(test)]` gating allowed.
- Consumes Plan 1 (`graph`, `bus`, `telemetry`) and Plan 2 (`board`): `TaskGraph`, `TaskId`, `TaskSpec`, `AgentKind`, `TaskState`, `EventBus`, `HiveEvent`, `AgentId`, `Blackboard`, `TaskResult`.
- The scheduler must NOT block: all waiting is `.await` on tokio primitives.

---

### Task 1: Agent abstraction + stubs

**Files:**
- Create: `crates/crew-hive/src/agent/mod.rs`
- Create: `crates/crew-hive/src/agent/stub.rs`
- Create: `crates/crew-hive/src/agent/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod agent;`)

**Interfaces:**
- Consumes: `crate::graph::{AgentKind, TaskId, TaskSpec}`, `crate::bus::{AgentId, EventBus, HiveEvent}`, `crate::board::TaskResult`.
- Produces (in `crate::agent`):
  - `pub struct AgentContext { pub agent: AgentId, pub task: TaskSpec, pub deps: Vec<TaskResult>, pub bus: EventBus }`
  - `pub trait Agent: Send + Sync { fn run(&self, ctx: AgentContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = TaskResult> + Send>>; }`
  - `pub trait AgentFactory: Send + Sync { fn make(&self, kind: &AgentKind) -> Box<dyn Agent>; }`
  - `pub struct StubAgent { pub fail: bool }` — implements `Agent`: emits `AgentSpawned`-independent token/output events and returns a `TaskResult` (success = `!fail`). Output = `format!("stub:{} deps={}", ctx.task.id.0, ctx.deps.len())`. On run it publishes `HiveEvent::OutputChunk { agent, text: "<output>\n" }` and `HiveEvent::TokenDelta { agent, input: 1, output: 1 }`; if `fail`, also `HiveEvent::Failed { agent, error: "stub failure".into() }`.
  - `pub struct StubFactory { pub fail_tasks: std::collections::HashSet<TaskId> }` — implements `AgentFactory`; `make` ignores the kind and returns a `StubAgent`. (Per-task failure is decided by the scheduler context, so `StubFactory::make` returns `StubAgent { fail: false }` by default; for failure tests use `FailingFactory` below.)
  - `pub struct FailingFactory { pub fail_tasks: std::collections::HashSet<TaskId> }` — `AgentFactory` whose agents fail for the configured task ids. Since `make` only sees the kind (not the task id), implement failure via a `StubAgent` variant that checks `ctx.task.id`: change `StubAgent` to `pub struct StubAgent { pub fail_ids: std::collections::HashSet<TaskId> }` and have `run` set `success = !ctx.task.id ∈ fail_ids`. Then `StubFactory` makes `StubAgent { fail_ids: <empty> }` and `FailingFactory` makes `StubAgent { fail_ids: self.fail_tasks.clone() }`. (Use this id-set design — it is what the scheduler tests rely on.)

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/agent/tests.rs`:

```rust
use super::*;
use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use std::collections::HashSet;

fn spec(id: u64) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: "t".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: String::new(),
    }
}

#[tokio::test]
async fn stub_agent_succeeds_and_emits() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();
    let agent = StubAgent { fail_ids: HashSet::new() };
    let ctx = AgentContext {
        agent: AgentId(0),
        task: spec(7),
        deps: vec![TaskResult { task: TaskId(1), output: "d".into(), success: true }],
        bus: bus.clone(),
    };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.task, TaskId(7));
    assert_eq!(result.output, "stub:7 deps=1");
    // at least one event was emitted
    assert!(matches!(rx.try_recv(), Ok(HiveEvent::OutputChunk { .. }) | Ok(HiveEvent::TokenDelta { .. })));
}

#[tokio::test]
async fn stub_agent_fails_for_configured_id() {
    let bus = EventBus::new(32);
    let mut ids = HashSet::new();
    ids.insert(TaskId(3));
    let agent = StubAgent { fail_ids: ids };
    let ctx = AgentContext { agent: AgentId(0), task: spec(3), deps: vec![], bus };
    let result = agent.run(ctx).await;
    assert!(!result.success);
}

#[test]
fn factory_makes_agents() {
    let f = StubFactory;
    let _a = f.make(&AgentKind::Api { system: None });
    let mut ids = HashSet::new();
    ids.insert(TaskId(1));
    let ff = FailingFactory { fail_tasks: ids };
    let _b = ff.make(&AgentKind::Pty { command: "sh".into(), args: vec![] });
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-hive agent::`
Expected: FAIL — `agent` items not defined.

- [ ] **Step 3: Implement the agent module**

Create `crates/crew-hive/src/agent/mod.rs` with `AgentContext`, `Agent`, `AgentFactory`, `StubFactory` (unit struct), `FailingFactory`, and declare `mod stub; #[cfg(test)] mod tests;`, re-exporting `StubAgent`.

```rust
//! Agent abstraction: a unit of work the scheduler runs. `Agent` is
//! object-safe (boxed future, no async-trait dep) so PTY/API/stub agents share
//! one interface. `AgentFactory` maps an `AgentKind` to a boxed agent.
mod stub;
#[cfg(test)]
mod tests;

pub use stub::StubAgent;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, TaskId, TaskSpec};

/// Everything an agent needs to do its task: its id, the task spec, the
/// already-gathered results of its dependencies, and the event bus.
pub struct AgentContext {
    pub agent: AgentId,
    pub task: TaskSpec,
    pub deps: Vec<TaskResult>,
    pub bus: EventBus,
}

/// A unit of work. Object-safe: `run` returns a boxed future so `Box<dyn Agent>`
/// works without the `async-trait` crate.
pub trait Agent: Send + Sync {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>>;
}

/// Maps a task's `AgentKind` to a concrete agent.
pub trait AgentFactory: Send + Sync {
    fn make(&self, kind: &AgentKind) -> Box<dyn Agent>;
}

/// Test factory: makes always-succeeding stub agents.
pub struct StubFactory;

impl AgentFactory for StubFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        Box::new(StubAgent { fail_ids: HashSet::new() })
    }
}

/// Test factory: makes stub agents that fail for the configured task ids.
pub struct FailingFactory {
    pub fail_tasks: HashSet<TaskId>,
}

impl AgentFactory for FailingFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        Box::new(StubAgent { fail_ids: self.fail_tasks.clone() })
    }
}
```

Create `crates/crew-hive/src/agent/stub.rs`:

```rust
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::graph::TaskId;

use super::{Agent, AgentContext};

/// A deterministic agent for headless tests: emits an output + token event and
/// returns a result whose success depends on whether its task id is in
/// `fail_ids`.
pub struct StubAgent {
    pub fail_ids: HashSet<TaskId>,
}

impl Agent for StubAgent {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let fail = self.fail_ids.contains(&ctx.task.id);
        Box::pin(async move {
            let output = format!("stub:{} deps={}", ctx.task.id.0, ctx.deps.len());
            ctx.bus.publish(HiveEvent::OutputChunk {
                agent: ctx.agent,
                text: format!("{output}\n"),
            });
            ctx.bus.publish(HiveEvent::TokenDelta { agent: ctx.agent, input: 1, output: 1 });
            if fail {
                ctx.bus.publish(HiveEvent::Failed {
                    agent: ctx.agent,
                    error: "stub failure".into(),
                });
            }
            TaskResult { task: ctx.task.id, output, success: !fail }
        })
    }
}
```

Add `pub mod agent;` to `lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-hive agent::`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets` (warning-free).

```bash
git add crates/crew-hive/src/agent crates/crew-hive/src/lib.rs
git commit -m "feat(hive): Agent trait + factory + headless stub agents"
```

---

### Task 2: Scheduler — execute a graph over a bounded pool

**Files:**
- Create: `crates/crew-hive/src/sched/mod.rs`
- Create: `crates/crew-hive/src/sched/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod sched;`)

**Interfaces:**
- Consumes: everything above + `crate::graph::{TaskGraph, TaskId, TaskState}`, `crate::board::Blackboard`, `crate::bus::{AgentId, EventBus, HiveEvent}`, `crate::agent::{AgentContext, AgentFactory}`.
- Produces (in `crate::sched`):
  - `pub struct RunOutcome { pub done: Vec<TaskId>, pub failed: Vec<TaskId>, pub cancelled: Vec<TaskId> }` — each sorted ascending. `Clone, Debug, PartialEq`.
  - `pub struct Scheduler { graph: TaskGraph, board: Blackboard, bus: EventBus, factory: std::sync::Arc<dyn AgentFactory>, concurrency: usize }` with:
    - `pub fn new(graph: TaskGraph, board: Blackboard, bus: EventBus, factory: Arc<dyn AgentFactory>, concurrency: usize) -> Self` (clamp concurrency to `>= 1`).
    - `pub async fn run(self) -> RunOutcome` — executes to completion: repeatedly spawns ready tasks (deps all `done`) onto a `JoinSet`, each gated by a `Semaphore` permit (cap = concurrency); on completion records the `TaskResult` to the board, marks `done`/`failed`, emits `TaskStateChanged`; cascades `Cancelled` to any task with a failed/cancelled dependency; terminates when no task is running and none remain ready.
- Behavior contract (tested):
  - All tasks of a valid DAG with succeeding agents end in `done`; results are in the blackboard; dependents receive their deps' results (via `gather`).
  - At most `concurrency` agents run simultaneously.
  - A failed task's transitive dependents are `cancelled` (never run); independent branches still complete.
  - Emits `AgentSpawned`, `TaskStateChanged{Running}` before run, and `TaskStateChanged{Done|Failed}` after; `Cancelled` for cascaded tasks.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/sched/tests.rs`:

```rust
use super::*;
use crate::agent::{FailingFactory, StubFactory};
use crate::board::Blackboard;
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use std::collections::HashSet;
use std::sync::Arc;

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

#[tokio::test]
async fn runs_linear_chain_to_completion() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[1])]).unwrap();
    let board = Blackboard::new();
    let sched = Scheduler::new(g, board.clone(), EventBus::new(64), Arc::new(StubFactory), 4);
    let out = sched.run().await;
    assert_eq!(out.done, vec![TaskId(0), TaskId(1), TaskId(2)]);
    assert!(out.failed.is_empty() && out.cancelled.is_empty());
    // results landed in the board
    assert_eq!(board.result_count().await, 3);
    // dependent saw its dep's result
    assert_eq!(board.get_result(TaskId(2)).await.unwrap().output, "stub:2 deps=1");
}

#[tokio::test]
async fn runs_diamond() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[0]), spec(3, &[1, 2])]).unwrap();
    let sched = Scheduler::new(g, Blackboard::new(), EventBus::new(64), Arc::new(StubFactory), 4);
    let out = sched.run().await;
    assert_eq!(out.done, vec![TaskId(0), TaskId(1), TaskId(2), TaskId(3)]);
}

#[tokio::test]
async fn respects_concurrency_cap() {
    use crate::agent::{Agent, AgentContext, AgentFactory};
    use crate::board::TaskResult;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // An agent that tracks peak concurrency via shared atomics.
    struct Counting {
        cur: Arc<AtomicUsize>,
        max: Arc<AtomicUsize>,
    }
    impl Agent for Counting {
        fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
            let cur = self.cur.clone();
            let max = self.max.clone();
            Box::pin(async move {
                let now = cur.fetch_add(1, Ordering::SeqCst) + 1;
                max.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                cur.fetch_sub(1, Ordering::SeqCst);
                TaskResult { task: ctx.task.id, output: String::new(), success: true }
            })
        }
    }
    struct CountingFactory {
        cur: Arc<AtomicUsize>,
        max: Arc<AtomicUsize>,
    }
    impl AgentFactory for CountingFactory {
        fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
            Box::new(Counting { cur: self.cur.clone(), max: self.max.clone() })
        }
    }

    let cur = Arc::new(AtomicUsize::new(0));
    let max = Arc::new(AtomicUsize::new(0));
    // 6 independent tasks, cap 2.
    let tasks: Vec<TaskSpec> = (0..6).map(|i| spec(i, &[])).collect();
    let g = TaskGraph::new(tasks).unwrap();
    let f = Arc::new(CountingFactory { cur: cur.clone(), max: max.clone() });
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), f, 2).run().await;
    assert_eq!(out.done.len(), 6);
    assert!(max.load(Ordering::SeqCst) <= 2, "peak concurrency {} exceeded cap 2", max.load(Ordering::SeqCst));
}

#[tokio::test]
async fn failure_cascades_cancel_to_dependents() {
    // 0 fails; 1 depends on 0 -> cancelled; 2 is independent -> done.
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[])]).unwrap();
    let mut fail = HashSet::new();
    fail.insert(TaskId(0));
    let f = Arc::new(FailingFactory { fail_tasks: fail });
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), f, 4).run().await;
    assert_eq!(out.failed, vec![TaskId(0)]);
    assert_eq!(out.cancelled, vec![TaskId(1)]);
    assert_eq!(out.done, vec![TaskId(2)]);
}

#[tokio::test]
async fn transitive_cancel() {
    // 0 fails -> 1 (dep 0) cancelled -> 2 (dep 1) cancelled.
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[1])]).unwrap();
    let mut fail = HashSet::new();
    fail.insert(TaskId(0));
    let f = Arc::new(FailingFactory { fail_tasks: fail });
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), f, 4).run().await;
    assert_eq!(out.failed, vec![TaskId(0)]);
    assert_eq!(out.cancelled, vec![TaskId(1), TaskId(2)]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-hive sched::`
Expected: FAIL — `Scheduler` not defined.

- [ ] **Step 3: Implement the scheduler**

Create `crates/crew-hive/src/sched/mod.rs`:

```rust
//! Scheduler: runs a `TaskGraph` to completion over a bounded pool of agents.
//! Ready tasks (deps all done) are spawned onto a `JoinSet`, each gated by a
//! `Semaphore` permit (the concurrency cap). Results land in the `Blackboard`;
//! state transitions emit on the `EventBus`; a failed/cancelled task cascades
//! cancellation to its dependents.
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::agent::{AgentContext, AgentFactory};
use crate::board::Blackboard;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{TaskGraph, TaskId, TaskState};

#[derive(Clone, Debug, PartialEq)]
pub struct RunOutcome {
    pub done: Vec<TaskId>,
    pub failed: Vec<TaskId>,
    pub cancelled: Vec<TaskId>,
}

pub struct Scheduler {
    graph: TaskGraph,
    board: Blackboard,
    bus: EventBus,
    factory: Arc<dyn AgentFactory>,
    concurrency: usize,
}

impl Scheduler {
    pub fn new(
        graph: TaskGraph,
        board: Blackboard,
        bus: EventBus,
        factory: Arc<dyn AgentFactory>,
        concurrency: usize,
    ) -> Self {
        Self { graph, board, bus, factory, concurrency: concurrency.max(1) }
    }

    pub async fn run(self) -> RunOutcome {
        let sem = Arc::new(Semaphore::new(self.concurrency));
        let mut done: HashSet<TaskId> = HashSet::new();
        let mut failed: HashSet<TaskId> = HashSet::new();
        let mut cancelled: HashSet<TaskId> = HashSet::new();
        let mut started: HashSet<TaskId> = HashSet::new();
        let mut joinset: JoinSet<(TaskId, crate::board::TaskResult)> = JoinSet::new();
        let mut next_agent: u64 = 0;

        loop {
            self.cascade_cancel(&done, &failed, &mut cancelled, &started);
            // Spawn every ready (deps all done), not-yet-started task.
            for id in self.graph.ready(&done) {
                if started.contains(&id) || cancelled.contains(&id) {
                    continue;
                }
                started.insert(id);
                let spec = self.graph.get(id).unwrap().clone();
                let agent_id = AgentId(next_agent);
                next_agent += 1;
                let agent = self.factory.make(&spec.agent);
                let bus = self.bus.clone();
                let board = self.board.clone();
                let sem = sem.clone();
                joinset.spawn(async move {
                    let _permit = sem.acquire_owned().await.expect("semaphore open");
                    let deps = board.gather(&spec.deps).await;
                    bus.publish(HiveEvent::AgentSpawned { agent: agent_id, task: spec.id });
                    bus.publish(HiveEvent::TaskStateChanged { task: spec.id, state: TaskState::Running });
                    let task_id = spec.id;
                    let ctx = AgentContext { agent: agent_id, task: spec, deps, bus };
                    (task_id, agent.run(ctx).await)
                });
            }

            if joinset.is_empty() {
                break;
            }

            if let Some(joined) = joinset.join_next().await {
                let (id, result) = joined.expect("agent task panicked");
                if result.success {
                    self.board.put_result(result).await;
                    done.insert(id);
                    self.bus.publish(HiveEvent::TaskStateChanged { task: id, state: TaskState::Done });
                } else {
                    failed.insert(id);
                    self.bus.publish(HiveEvent::TaskStateChanged { task: id, state: TaskState::Failed });
                }
            }
        }

        RunOutcome {
            done: sorted(done),
            failed: sorted(failed),
            cancelled: sorted(cancelled),
        }
    }

    /// Mark every not-started task with a failed/cancelled dependency as
    /// cancelled (transitively, since newly-cancelled tasks feed the next pass
    /// via the scheduler loop).
    fn cascade_cancel(
        &self,
        done: &HashSet<TaskId>,
        failed: &HashSet<TaskId>,
        cancelled: &mut HashSet<TaskId>,
        started: &HashSet<TaskId>,
    ) {
        loop {
            let mut changed = false;
            for t in self.graph.tasks() {
                if done.contains(&t.id)
                    || failed.contains(&t.id)
                    || cancelled.contains(&t.id)
                    || started.contains(&t.id)
                {
                    continue;
                }
                if t.deps.iter().any(|d| failed.contains(d) || cancelled.contains(d)) {
                    cancelled.insert(t.id);
                    self.bus.publish(HiveEvent::TaskStateChanged {
                        task: t.id,
                        state: TaskState::Cancelled,
                    });
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }
}

fn sorted(set: HashSet<TaskId>) -> Vec<TaskId> {
    let mut v: Vec<TaskId> = set.into_iter().collect();
    v.sort_unstable();
    v
}
```

If `mod.rs` exceeds 200 lines, move `cascade_cancel` + `sorted` into a `sched/cancel.rs` submodule. (Estimate ~150 lines — likely fine.)

Add `pub mod sched;` to `lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-hive sched::`
Expected: PASS (5 tests). The concurrency-cap test uses real time (20ms sleeps) — it should still complete in well under a second.

- [ ] **Step 5: Commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets` (warning-free).

```bash
git add crates/crew-hive/src/sched crates/crew-hive/src/lib.rs
git commit -m "feat(hive): tokio DAG scheduler over bounded agent pool"
```

---

### Task 3: Public API surface + integration test + crate docs

Expose a clean top-level entry point and prove the whole engine works end-to-end with a realistic graph; document the crate.

**Files:**
- Modify: `crates/crew-hive/src/lib.rs` (curated re-exports + crate-level docs)
- Create: `crates/crew-hive/tests/engine.rs` (integration test using only the public API)

**Interfaces:**
- Produces (re-exported at crate root): `pub use graph::{TaskGraph, TaskSpec, TaskId, AgentKind, ModelTier, TaskState, GraphError};`, `pub use bus::{EventBus, HiveEvent, AgentId};`, `pub use board::{Blackboard, TaskResult, BlackboardSnapshot};`, `pub use telemetry::{Fleet, AgentTelemetry, FleetTotals};`, `pub use agent::{Agent, AgentContext, AgentFactory, StubAgent};`, `pub use sched::{Scheduler, RunOutcome};`. (Re-export only the public API; keep `StubAgent`/`StubFactory` available for downstream tests but `FailingFactory` may stay test-internal — if exporting it causes an unused warning, leave it unexported.)

- [ ] **Step 1: Write the failing integration test**

Create `crates/crew-hive/tests/engine.rs`:

```rust
//! End-to-end: build a graph, run it through the scheduler with stub agents,
//! and drive a telemetry Fleet from the bus — using ONLY the public API.
use crew_hive::{
    AgentKind, Blackboard, EventBus, Fleet, ModelTier, Scheduler, StubAgent, TaskGraph, TaskId,
    TaskSpec, TaskState,
};
use std::sync::Arc;

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

// A factory exported for downstream use: build via the public StubAgent.
struct Stubs;
impl crew_hive::AgentFactory for Stubs {
    fn make(&self, _k: &AgentKind) -> Box<dyn crew_hive::Agent> {
        Box::new(StubAgent { fail_ids: std::collections::HashSet::new() })
    }
}

#[tokio::test]
async fn end_to_end_fan_out_fan_in() {
    let g = TaskGraph::new(vec![
        spec(0, &[]),
        spec(1, &[0]),
        spec(2, &[0]),
        spec(3, &[1, 2]),
    ])
    .unwrap();
    let board = Blackboard::new();
    let bus = EventBus::new(256);

    // Drive telemetry from the bus concurrently.
    let mut rx = bus.subscribe();
    let collector = tokio::spawn(async move {
        let mut fleet = Fleet::new();
        while let Ok(ev) = rx.recv().await {
            fleet.apply(&ev);
        }
        fleet
    });

    let out = Scheduler::new(g, board.clone(), bus.clone(), Arc::new(Stubs), 8).run().await;
    drop(bus); // close the channel so the collector finishes
    let fleet = collector.await.unwrap();

    assert_eq!(out.done, vec![TaskId(0), TaskId(1), TaskId(2), TaskId(3)]);
    assert_eq!(board.result_count().await, 4);
    // every task reached Done in telemetry
    let totals = fleet.totals();
    assert_eq!(totals.done, 4);
    assert_eq!(totals.failed, 0);
    // the fan-in task saw both deps
    assert_eq!(board.get_result(TaskId(3)).await.unwrap().output, "stub:3 deps=2");
    let _ = TaskState::Done; // type is part of the public surface
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-hive --test engine`
Expected: FAIL — re-exports not present at crate root (e.g. `crew_hive::Scheduler` unresolved).

- [ ] **Step 3: Add re-exports + crate docs**

Update `crates/crew-hive/src/lib.rs` with the crate-level doc comment and the curated `pub use` re-exports listed in Interfaces. Keep the `pub mod` declarations too (so module paths still work).

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p crew-hive --test engine`
Expected: PASS.

- [ ] **Step 5: Full workspace gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets` (all crew-hive unit + integration tests pass; workspace warning-free).

```bash
git add crates/crew-hive/src/lib.rs crates/crew-hive/tests/engine.rs
git commit -m "feat(hive): public API re-exports + end-to-end engine integration test"
```

---

## Self-Review

- **Spec coverage:** "Scheduler — a tokio-based DAG executor: runs ready nodes up to a bounded concurrency limit, backpressure, fan-in waits for deps, cancellation; Agent Pool — lightweight agent handles multiplexed over a bounded pool." → `Scheduler::run` (JoinSet + Semaphore cap = backpressure; `ready()` = fan-in; `cascade_cancel` = failure→cancel), `Agent`/`AgentFactory` (the pool's unit), blackboard `gather` feeding deps. Retries are intentionally deferred to the later governance plan (YAGNI here). ✅
- **Placeholder scan:** Tasks 1–2 complete code incl. the concurrency-cap test harness; Task 3 complete integration test + explicit re-export list. ✅
- **Type consistency:** `Agent`/`AgentContext`/`AgentFactory`/`StubAgent`/`Scheduler`/`RunOutcome` consistent across tasks; `fail_ids: HashSet<TaskId>` design used by both factories and the scheduler failure tests. ✅
- **Concurrency correctness:** Semaphore caps execution; JoinSet awaits completions; `cascade_cancel` runs to fixpoint each loop so transitive cancellation is covered; termination when `joinset.is_empty()` and no ready tasks. ✅
- **No new deps / no async-trait / no GUI / no LLM.** ✅
- **Object safety:** `Agent::run` returns `Pin<Box<dyn Future + Send>>` → `Box<dyn Agent>` is valid. ✅

## Where this sits

Third engine plan — the executor. With this, crew-hive can run an arbitrary DAG of agents to completion headlessly. Next: the **Planner** (turn a goal into a `TaskGraph`; LLM-backed behind a provider trait, stub-tested) and the **native API agent** (a real `Agent` impl that calls an LLM via reqwest) — those two need an API key only for live testing.
```
