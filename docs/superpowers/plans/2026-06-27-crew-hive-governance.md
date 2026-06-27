# crew-hive Batch Mode + Budget Governance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Complete the unified substrate's second mode (parallel job fan-out) and add cost governance: a batch-graph builder, cooperative scheduler cancellation, and a budget governor that trips cancellation when fleet cost crosses a cap. All headless-testable.

**Architecture:** Three additions to `crew-hive`. `batch` builds a flat `TaskGraph` (each job an independent, dependency-free task) from a list of `Job`s — the "many parallel jobs" mode of the same engine. The `Scheduler` gains a cooperative cancel flag (`Arc<AtomicBool>`): when set, it stops spawning new tasks, marks unstarted tasks `Cancelled`, and drains in-flight agents — graceful shutdown without killing running work. `govern` provides a `Budget` and an async `budget_governor` that subscribes to the event bus, accumulates cost via a `Fleet`, and flips the cancel flag once `micros_usd` exceeds the cap. Together: point the engine at thousands of jobs with a hard cost ceiling.

**Tech Stack:** Rust, `tokio` (sync::AtomicBool via std, broadcast), `cargo test` + `tokio::test`. No new deps.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total.
- **No new dependencies.** Use `std::sync::atomic::{AtomicBool, Ordering}` + `std::sync::Arc`.
- Cancellation is cooperative and graceful: never abort running agents; stop *new* dispatch and cancel *unstarted* tasks.
- crew-hive depends on no other crew crate.
- Dead code removed, not suppressed; `#[cfg(test)]` gating allowed.
- Consumes: `crate::graph::{TaskGraph, TaskSpec, TaskId, AgentKind, ModelTier, GraphError}`, `crate::bus::{EventBus, HiveEvent}`, `crate::telemetry::Fleet`, and the existing `crate::sched::Scheduler`.

---

### Task 1: Batch graph builder

**Files:**
- Create: `crates/crew-hive/src/batch/mod.rs`
- Create: `crates/crew-hive/src/batch/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod batch;` + re-export `Job`, `batch_graph`)

**Interfaces:**
- Produces:
  - `pub struct Job { pub title: String, pub prompt: String, pub tier: ModelTier }` — `Clone, Debug`.
  - `pub fn batch_graph(jobs: Vec<Job>) -> Result<TaskGraph, GraphError>` — builds a flat graph: job `i` → `TaskSpec { id: TaskId(i as u64), title, agent: AgentKind::Api{system:None}, model: tier, deps: vec![], prompt }`. Empty input → an empty graph (`TaskGraph::new(vec![])`).

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/batch/tests.rs`:
```rust
use super::*;
use crate::graph::{ModelTier, TaskId};

fn job(p: &str, tier: ModelTier) -> Job {
    Job { title: p.into(), prompt: p.into(), tier }
}

#[test]
fn batch_graph_is_flat_and_independent() {
    let g = batch_graph(vec![
        job("a", ModelTier::Cheap),
        job("b", ModelTier::Standard),
        job("c", ModelTier::Capable),
    ])
    .unwrap();
    assert_eq!(g.len(), 3);
    // all tasks are roots (no deps) -> all ready immediately
    let ready = g.ready(&std::collections::HashSet::new());
    assert_eq!(ready, vec![TaskId(0), TaskId(1), TaskId(2)]);
    assert_eq!(g.get(TaskId(2)).unwrap().model, ModelTier::Capable);
}

#[test]
fn batch_graph_empty_ok() {
    let g = batch_graph(vec![]).unwrap();
    assert!(g.is_empty());
}
```

- [ ] **Step 2: Run fail → implement → pass**

Run `cargo test -p crew-hive batch::` (FAIL), implement `batch/mod.rs`, add `pub mod batch;` + re-exports to lib.rs, run again (PASS). Keep ≤ 200 lines.

- [ ] **Step 3: Lint + commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets`.
```bash
git add crates/crew-hive/src/batch crates/crew-hive/src/lib.rs
git commit -m "feat(hive): batch_graph — flat parallel-job task graph"
```

---

### Task 2: Cooperative scheduler cancellation

**Files:**
- Modify: `crates/crew-hive/src/sched/mod.rs` (add a cancel flag)
- Modify: `crates/crew-hive/src/sched/tests.rs` (append a cancellation test)

**Interfaces:**
- Produces (added to `Scheduler`):
  - `pub fn with_cancel(self, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self` — attaches a shared cancel flag (builder-style; default is a never-set flag).
  - Behavior: in `run`, at the top of each loop iteration, if `cancel.load(Relaxed)` is true, stop spawning new tasks and mark every not-started, not-done, not-failed task `Cancelled` (emit `TaskStateChanged{Cancelled}`); continue draining the `JoinSet` so in-flight agents finish and their results are recorded. The run ends when the JoinSet empties.
- Contract (tested): with the flag set before `run`, all tasks end `cancelled` (none run); with the flag set mid-run, already-running tasks complete (`done`) and unstarted tasks end `cancelled`.

- [ ] **Step 1: Write the failing test**

Append to `crates/crew-hive/src/sched/tests.rs`:
```rust
#[tokio::test]
async fn cancel_before_run_cancels_everything() {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[]), spec(2, &[])]).unwrap();
    let cancel = Arc::new(AtomicBool::new(true));
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), Arc::new(StubFactory), 4)
        .with_cancel(cancel)
        .run()
        .await;
    assert!(out.done.is_empty());
    assert_eq!(out.cancelled, vec![TaskId(0), TaskId(1), TaskId(2)]);
}

#[tokio::test]
async fn cancel_mid_run_drains_inflight() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use crate::agent::{Agent, AgentContext, AgentFactory};
    use crate::board::TaskResult;
    use std::future::Future;
    use std::pin::Pin;

    // Agent that flips cancel after starting, then sleeps briefly and succeeds.
    struct Flip { cancel: Arc<AtomicBool> }
    impl Agent for Flip {
        fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
            let cancel = self.cancel.clone();
            Box::pin(async move {
                cancel.store(true, Ordering::Relaxed);
                tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                TaskResult { task: ctx.task.id, output: String::new(), success: true }
            })
        }
    }
    struct FlipFactory { cancel: Arc<AtomicBool> }
    impl AgentFactory for FlipFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> { Box::new(Flip { cancel: self.cancel.clone() }) }
    }

    // One root (runs and flips cancel) + one dependent (should be cancelled).
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0])]).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let out = Scheduler::new(g, Blackboard::new(), EventBus::new(64), Arc::new(FlipFactory { cancel: cancel.clone() }), 1)
        .with_cancel(cancel)
        .run()
        .await;
    assert_eq!(out.done, vec![TaskId(0)]);     // in-flight completed
    assert_eq!(out.cancelled, vec![TaskId(1)]); // unstarted dependent cancelled
}
```

- [ ] **Step 2: Run fail → implement → pass**

Add a `cancel: Arc<AtomicBool>` field to `Scheduler` (default `Arc::new(AtomicBool::new(false))` in `new`), the `with_cancel` builder, and the cancel check in `run`'s loop. When cancelled, mark all not-{started/done/failed/cancelled} tasks `Cancelled` (reuse the cascade emit pattern) and skip the spawn loop; still `join_next().await` until empty. Keep `sched/mod.rs` ≤ 200 (it's near the cap — if the addition crosses 200, move `cascade_cancel`/`sorted`/the cancel-marking into a `sched/cancel.rs` submodule as the earlier plan noted). Run `cargo test -p crew-hive sched::` PASS.

- [ ] **Step 3: Lint + commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets`.
```bash
git add crates/crew-hive/src/sched
git commit -m "feat(hive): cooperative scheduler cancellation (drain in-flight)"
```

---

### Task 3: Budget governor

**Files:**
- Create: `crates/crew-hive/src/govern/mod.rs`
- Create: `crates/crew-hive/src/govern/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod govern;` + re-export `Budget`, `budget_governor`)

**Interfaces:**
- Consumes: `crate::bus::{EventBus, HiveEvent}`, `crate::telemetry::Fleet`.
- Produces:
  - `pub struct Budget { pub max_micros_usd: u64 }` — `Copy, Clone, Debug`.
  - `pub async fn budget_governor(bus: &EventBus, budget: Budget, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>)` — subscribes to the bus, applies each `HiveEvent` to a `Fleet`, and when `fleet.totals().micros_usd > budget.max_micros_usd` sets `cancel` to `true` and returns. Returns when the bus closes (all senders dropped) even if the cap is never hit.
- Contract (tested): publishing cost events that exceed the cap flips the cancel flag; staying under the cap leaves it unset (governor returns when the bus closes).

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/govern/tests.rs`:
```rust
use super::*;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::TaskId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn governor_trips_cancel_over_budget() {
    let bus = EventBus::new(64);
    let cancel = Arc::new(AtomicBool::new(false));
    let c2 = cancel.clone();
    let bus2 = bus.clone();
    let gov = tokio::spawn(async move {
        budget_governor(&bus2, Budget { max_micros_usd: 1000 }, c2).await;
    });
    bus.publish(HiveEvent::AgentSpawned { agent: AgentId(0), task: TaskId(0) });
    bus.publish(HiveEvent::CostDelta { agent: AgentId(0), micros_usd: 1500 });
    drop(bus); // close so the governor returns after processing
    gov.await.unwrap();
    assert!(cancel.load(Ordering::Relaxed));
}

#[tokio::test]
async fn governor_stays_unset_under_budget() {
    let bus = EventBus::new(64);
    let cancel = Arc::new(AtomicBool::new(false));
    let c2 = cancel.clone();
    let bus2 = bus.clone();
    let gov = tokio::spawn(async move {
        budget_governor(&bus2, Budget { max_micros_usd: 10_000 }, c2).await;
    });
    bus.publish(HiveEvent::AgentSpawned { agent: AgentId(0), task: TaskId(0) });
    bus.publish(HiveEvent::CostDelta { agent: AgentId(0), micros_usd: 500 });
    drop(bus);
    gov.await.unwrap();
    assert!(!cancel.load(Ordering::Relaxed));
}
```

- [ ] **Step 2: Run fail → implement → pass**

Implement `govern/mod.rs`: `budget_governor` calls `bus.subscribe()`, loops `rx.recv().await`, applies each event to a local `Fleet`, checks the cap, and breaks/returns on cap-exceeded or on `Err` (channel closed/lagged → on `Lagged` continue, on `Closed` return). Add `pub mod govern;` + re-exports. Run `cargo test -p crew-hive govern::` PASS. Keep ≤ 200 lines.

- [ ] **Step 3: Full gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets`.
```bash
git add crates/crew-hive/src/govern crates/crew-hive/src/lib.rs
git commit -m "feat(hive): budget governor — trip scheduler cancel on cost cap"
```

---

## Self-Review

- **Spec coverage:** "Batch/queue mode (the many parallel jobs half of the unified substrate)" → `batch_graph`. "Cost/model governance: budget caps" → `Budget` + `budget_governor` + scheduler `with_cancel`. Model tiering already exists (`ModelTier`, set per `Job`). ✅
- **Placeholder scan:** Task 1 + 3 complete interfaces/tests; Task 2 complete test + precise behavior. ✅
- **Cancellation is graceful:** drains in-flight, cancels only unstarted — tested both pre-run and mid-run. ✅
- **No new deps; no GUI; no LLM.** ✅
- **File sizes:** new files small; `sched/mod.rs` is near 200 — split note included. ✅

## Where this sits

Completes the unified substrate (single-goal decomposition + parallel-job batch) with a hard cost ceiling. Last engine plan: **remote spill + sidecar bridge** — a JSON protocol + a `RemoteAgent`/worker loopback so the scheduler can dispatch tasks to out-of-process workers (and external engines like LangGraph) over a wire, testable in-process.
