# crew-hive Blackboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add a concurrent shared-state store (`Blackboard`) to `crew-hive` so agents publish their task results and read upstream dependencies' results — the mechanism that lets a fan-out of agents merge results upward (replacing the fragile file/sentinel convention).

**Architecture:** One module `board` in `crew-hive`. `Blackboard` is a cheap-to-clone handle wrapping `Arc<tokio::sync::RwLock<Inner>>`; many agent tasks hold clones and read/write concurrently. It stores per-task `TaskResult`s (output + success) keyed by `TaskId`, plus a free-form `String→String` artifact map. `gather(deps)` collects upstream results in dependency order to assemble a downstream agent's prompt. `snapshot()` produces a serde-able copy for the future remote/sidecar bridge. Purely additive to crew-hive.

**Tech Stack:** Rust, `tokio::sync::RwLock`, `serde`/`serde_json`, `tokio::test`. All existing deps.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total.
- **No new dependencies** — only crew-hive's existing deps (tokio, serde, serde_json).
- Boundary types (`TaskResult`, `BlackboardSnapshot`) derive `Serialize, Deserialize`.
- crew-hive depends on no other crew crate.
- Dead code removed, not suppressed; `#[cfg(test)]` gating allowed.
- Consumes `crate::graph::TaskId`.

---

### Task 1: Blackboard core — results + dependency gather

**Files:**
- Create: `crates/crew-hive/src/board/mod.rs`
- Create: `crates/crew-hive/src/board/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod board;`)

**Interfaces:**
- Consumes: `crate::graph::TaskId`.
- Produces (in `crate::board`):
  - `pub struct TaskResult { pub task: TaskId, pub output: String, pub success: bool }` — `Clone, Debug, Serialize, Deserialize, PartialEq`.
  - `pub struct Blackboard { /* Arc<RwLock<Inner>> */ }` — `Clone`, `Default`.
    - `pub fn new() -> Self`
    - `pub async fn put_result(&self, result: TaskResult)` — inserts/overwrites the result for `result.task`.
    - `pub async fn get_result(&self, task: TaskId) -> Option<TaskResult>`
    - `pub async fn gather(&self, deps: &[TaskId]) -> Vec<TaskResult>` — returns the present results for `deps`, in the order given (skipping any not yet present).
    - `pub async fn result_count(&self) -> usize`

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/board/tests.rs`:

```rust
use super::*;
use crate::graph::TaskId;

fn res(task: u64, out: &str, ok: bool) -> TaskResult {
    TaskResult { task: TaskId(task), output: out.into(), success: ok }
}

#[tokio::test]
async fn put_then_get_result() {
    let b = Blackboard::new();
    b.put_result(res(1, "hello", true)).await;
    assert_eq!(b.get_result(TaskId(1)).await, Some(res(1, "hello", true)));
    assert_eq!(b.get_result(TaskId(2)).await, None);
    assert_eq!(b.result_count().await, 1);
}

#[tokio::test]
async fn put_overwrites_same_task() {
    let b = Blackboard::new();
    b.put_result(res(1, "old", true)).await;
    b.put_result(res(1, "new", false)).await;
    assert_eq!(b.get_result(TaskId(1)).await.unwrap().output, "new");
    assert_eq!(b.result_count().await, 1);
}

#[tokio::test]
async fn gather_returns_present_deps_in_order() {
    let b = Blackboard::new();
    b.put_result(res(2, "two", true)).await;
    b.put_result(res(0, "zero", true)).await;
    // dep 1 absent; expect [0, 2] in the requested order, skipping 1.
    let got = b.gather(&[TaskId(0), TaskId(1), TaskId(2)]).await;
    let tasks: Vec<TaskId> = got.iter().map(|r| r.task).collect();
    assert_eq!(tasks, vec![TaskId(0), TaskId(2)]);
}

#[tokio::test]
async fn clones_share_state() {
    let b = Blackboard::new();
    let b2 = b.clone();
    b.put_result(res(5, "x", true)).await;
    // The clone sees the write — shared Arc.
    assert_eq!(b2.get_result(TaskId(5)).await.unwrap().output, "x");
}

#[tokio::test]
async fn concurrent_writers_all_land() {
    let b = Blackboard::new();
    let mut handles = Vec::new();
    for i in 0..50u64 {
        let bc = b.clone();
        handles.push(tokio::spawn(async move {
            bc.put_result(res(i, "v", true)).await;
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(b.result_count().await, 50);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-hive board::`
Expected: FAIL — `board` items not defined.

- [ ] **Step 3: Implement the blackboard core**

Create `crates/crew-hive/src/board/mod.rs`:

```rust
//! Blackboard: a concurrent shared store where agents publish their task
//! results and read upstream dependencies' results, so a fan-out of agents can
//! merge results upward. Cheap to clone (shared `Arc<RwLock<_>>`).
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::graph::TaskId;

/// The result an agent publishes for its task.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskResult {
    pub task: TaskId,
    pub output: String,
    pub success: bool,
}

#[derive(Default)]
struct Inner {
    results: HashMap<TaskId, TaskResult>,
    artifacts: HashMap<String, String>,
}

/// Shared, cloneable handle to the blackboard.
#[derive(Clone, Default)]
pub struct Blackboard {
    inner: Arc<RwLock<Inner>>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn put_result(&self, result: TaskResult) {
        self.inner.write().await.results.insert(result.task, result);
    }

    pub async fn get_result(&self, task: TaskId) -> Option<TaskResult> {
        self.inner.read().await.results.get(&task).cloned()
    }

    /// Present results for `deps`, in the order given (absent ones skipped).
    pub async fn gather(&self, deps: &[TaskId]) -> Vec<TaskResult> {
        let g = self.inner.read().await;
        deps.iter().filter_map(|d| g.results.get(d).cloned()).collect()
    }

    pub async fn result_count(&self) -> usize {
        self.inner.read().await.results.len()
    }
}
```

(The `artifacts` field is unused until Task 2 — that is one transient dead-code situation. To avoid a warning in the interim, Task 2 follows immediately in the same plan/branch and consumes it. If `cargo clippy` flags `artifacts` as dead between Task 1 and Task 2, that is the known cross-task deferral — do NOT add `#[allow]`; note it and proceed. The whole-plan gate runs after Task 2 when it is consumed.)

Add `pub mod board;` to `lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-hive board::`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

Run: `cargo fmt`. (Do not gate on clippy here if the only warning is the transient `artifacts` dead-code consumed in Task 2; note it.)

```bash
git add crates/crew-hive/src/board crates/crew-hive/src/lib.rs
git commit -m "feat(hive): blackboard core — task results + dependency gather"
```

---

### Task 2: Artifacts + serde snapshot

Free-form `String→String` artifacts and a serializable snapshot of the whole board (for the future remote/sidecar bridge and the swarm view).

**Files:**
- Modify: `crates/crew-hive/src/board/mod.rs` (add artifact methods + snapshot)
- Modify: `crates/crew-hive/src/board/tests.rs` (append tests)

**Interfaces:**
- Produces (added to `Blackboard`):
  - `pub async fn put_artifact(&self, key: impl Into<String>, value: impl Into<String>)`
  - `pub async fn get_artifact(&self, key: &str) -> Option<String>`
  - `pub async fn snapshot(&self) -> BlackboardSnapshot`
- New type:
  - `pub struct BlackboardSnapshot { pub results: Vec<TaskResult>, pub artifacts: Vec<(String, String)> }` — `Clone, Debug, Serialize, Deserialize, PartialEq`. `results` sorted by `task` id ascending and `artifacts` sorted by key, for deterministic output.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-hive/src/board/tests.rs`:

```rust
#[tokio::test]
async fn put_then_get_artifact() {
    let b = Blackboard::new();
    b.put_artifact("plan", "decompose into 3").await;
    assert_eq!(b.get_artifact("plan").await.as_deref(), Some("decompose into 3"));
    assert_eq!(b.get_artifact("missing").await, None);
}

#[tokio::test]
async fn snapshot_is_sorted_and_roundtrips() {
    let b = Blackboard::new();
    b.put_result(res(2, "two", true)).await;
    b.put_result(res(0, "zero", true)).await;
    b.put_artifact("z", "1").await;
    b.put_artifact("a", "2").await;
    let snap = b.snapshot().await;
    assert_eq!(snap.results.iter().map(|r| r.task).collect::<Vec<_>>(), vec![TaskId(0), TaskId(2)]);
    assert_eq!(snap.artifacts.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(), vec!["a".to_string(), "z".to_string()]);
    let json = serde_json::to_string(&snap).unwrap();
    let back: BlackboardSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(snap, back);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-hive board::`
Expected: FAIL — artifact methods / `BlackboardSnapshot` not defined.

- [ ] **Step 3: Implement artifacts + snapshot**

Add to `board/mod.rs`: the `BlackboardSnapshot` struct (with derives), and the three methods. `snapshot` reads the lock, collects results into a Vec sorted by `task`, and artifacts into a Vec sorted by key. Keep `mod.rs` ≤ 200 lines (split a `snapshot.rs` submodule if needed).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-hive board::`
Expected: PASS.

- [ ] **Step 5: Full gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets` (warning-free — `artifacts` is now consumed).

```bash
git add crates/crew-hive/src/board
git commit -m "feat(hive): blackboard artifacts + serde snapshot"
```

---

## Self-Review

- **Spec coverage:** "Blackboard — shared state/artifact store so agents merge results upward; structured result contracts replace grep-for-a-sentinel." → `TaskResult` (structured), `gather(deps)` (merge upstream), artifacts (free-form shared state), snapshot (boundary serialization). ✅
- **Placeholder scan:** Task 1 complete code; Task 2 complete interfaces + tests + precise snapshot rules. ✅
- **Type consistency:** `Blackboard`/`TaskResult`/`BlackboardSnapshot` consistent; `gather(&[TaskId])` signature stable for the scheduler. ✅
- **Concurrency:** `Arc<RwLock>` shared across clones; concurrent-writers test covers it. ✅
- **No new deps / no GUI / no LLM.** ✅

## Where this sits

Second engine plan. Next: the **tokio DAG scheduler + agent pool** — it consumes `TaskGraph` + `ready()` (Plan 1), spawns agents up to a concurrency cap, has each agent read deps via `Blackboard::gather` and write its `TaskResult` via `put_result`, and emits `HiveEvent`s on the bus.
```
