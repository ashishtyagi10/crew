# Long-running agents (concurrent background tasks) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Let the crew broker run several agent tasks in the background at once (each addressable and stoppable), instead of one-at-a-time.

**Architecture:** A new `broker/tasks.rs` registry owns a `Vec` of running tasks, each with a monotonic id and its OWN cancel flag. `run_broker_stdio` spawns a fresh worker per Send (up to a cap) instead of rejecting when busy; `Session::snapshot` now hands each task an independent cancel flag so `/stop #N` cancels only that task. Streamed `Message` events carry `meta = "task:<id>"`; crew-app tags them with a dim `#N` chip.

**Tech Stack:** Rust, `std::thread`, `Arc<AtomicBool>`, existing `Broker`/`Session` in crew-plugin; crew-app cell rendering.

**Spec:** docs/superpowers/specs/2026-07-03-long-running-agents-design.md

## Global Constraints
- crew-plugin broker files stay focused; tests in sibling `*_tests.rs` or in-file `#[cfg(test)] mod tests` per neighbors.
- Concurrency cap: `CREW_MAX_TASKS` env (default 4). Over cap → reject the Send.
- `CREW_BROKER_MOCK_REPLY` set = mock provider (deterministic tests, no API keys).
- Message task tag: `meta = "task:<id>"` (id is the u64). No PluginEvent schema change.
- Commit messages end with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`. `cargo fmt` before each commit (pre-commit runs fmt + check).

---

### Task 1: `broker/tasks.rs` — the task registry

**Files:**
- Create: `crates/crew-plugin/src/broker/tasks.rs`
- Modify: `crates/crew-plugin/src/broker/mod.rs` (add `pub(crate) mod tasks;` beside `mod session;`)

**Interfaces produced (used by Task 3):**
- `pub(crate) struct Tasks { … }` with:
  - `fn new() -> Tasks`
  - `fn max() -> usize` (reads `CREW_MAX_TASKS`, default 4)
  - `fn admit(&self) -> bool` (live count < max, after reap)
  - `fn register(&mut self, label, cancel, handle, now) -> u64` (one-step, for tests) and the two-step pair `fn reserve(&mut self) -> u64` + `fn attach(&mut self, id, label, cancel, handle, now)` (stdio needs the id before the handle exists)
  - `fn reap(&mut self)` (drop finished; `JoinHandle::is_finished`)
  - `fn cancel(&self, id: u64) -> bool` (trip that task's flag; false if unknown)
  - `fn cancel_all(&self) -> usize` (trip all; returns count)
  - `fn len(&self) -> usize`
  - `fn describe(&self, now: Instant) -> Vec<String>` (e.g. `"#3 · <label> · 2m"`)

- [ ] **Step 1: Write the failing tests**

`crates/crew-plugin/src/broker/tasks.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    fn spawn_flag() -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
        let flag = Arc::new(AtomicBool::new(false));
        let f = Arc::clone(&flag);
        // A thread that runs until its flag trips, so is_finished() is false
        // until we cancel it — lets us test reap/cancel deterministically.
        let h = std::thread::spawn(move || {
            while !f.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        (flag, h)
    }

    #[test]
    fn register_assigns_increasing_ids() {
        let mut t = Tasks::new();
        let (c1, h1) = spawn_flag();
        let (c2, h2) = spawn_flag();
        let id1 = t.register("a".into(), Arc::clone(&c1), h1, Instant::now());
        let id2 = t.register("b".into(), Arc::clone(&c2), h2, Instant::now());
        assert_eq!((id1, id2), (1, 2));
        assert_eq!(t.len(), 2);
        t.cancel_all();
    }

    #[test]
    fn cancel_trips_only_that_task() {
        let mut t = Tasks::new();
        let (c1, h1) = spawn_flag();
        let (c2, h2) = spawn_flag();
        let id1 = t.register("a".into(), Arc::clone(&c1), h1, Instant::now());
        t.register("b".into(), Arc::clone(&c2), h2, Instant::now());
        assert!(t.cancel(id1));
        assert!(c1.load(Ordering::Relaxed));
        assert!(!c2.load(Ordering::Relaxed));
        assert!(!t.cancel(999), "unknown id");
        t.cancel_all();
    }

    #[test]
    fn reap_drops_finished_tasks() {
        let mut t = Tasks::new();
        let (c1, h1) = spawn_flag();
        t.register("a".into(), Arc::clone(&c1), h1, Instant::now());
        c1.store(true, Ordering::Relaxed); // let the thread exit
        // Give it a moment, then reap.
        std::thread::sleep(std::time::Duration::from_millis(50));
        t.reap();
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn admit_respects_the_cap() {
        // admit() is count < max; with the default max (>=1) an empty registry admits.
        let t = Tasks::new();
        assert!(t.admit());
    }

    #[test]
    fn describe_lists_id_and_label() {
        let mut t = Tasks::new();
        let (c1, h1) = spawn_flag();
        t.register("refactor".into(), Arc::clone(&c1), h1, Instant::now());
        let d = t.describe(Instant::now());
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("#1") && d[0].contains("refactor"), "{}", d[0]);
        t.cancel_all();
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-plugin tasks 2>&1 | tail -5`
Expected: compile error — module/types missing.

- [ ] **Step 3: Implement `tasks.rs`**

```rust
//! The broker's running-task registry: several agent tasks run concurrently,
//! each on its own worker thread with its own cancel flag and a monotonic id.
//! Replaces the old single-worker/`busy`/`cancel` model in `run_broker_stdio`.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

/// One running background task.
struct Task {
    id: u64,
    label: String,
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<()>,
    started: Instant,
}

/// All background tasks currently running.
pub(crate) struct Tasks {
    next_id: u64,
    running: Vec<Task>,
}

impl Tasks {
    pub(crate) fn new() -> Self {
        Tasks {
            next_id: 0,
            running: Vec::new(),
        }
    }

    /// Concurrency cap. `CREW_MAX_TASKS` overrides (default 4, floored at 1).
    pub(crate) fn max() -> usize {
        std::env::var("CREW_MAX_TASKS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4usize)
            .max(1)
    }

    /// Whether another task may start (live count below the cap).
    pub(crate) fn admit(&self) -> bool {
        self.running.len() < Self::max()
    }

    /// Register a spawned task in one step; returns its new id. (Used by the
    /// unit tests; `stdio` uses `reserve` + `attach` because it needs the id
    /// before the `JoinHandle` exists.)
    pub(crate) fn register(
        &mut self,
        label: String,
        cancel: Arc<AtomicBool>,
        handle: JoinHandle<()>,
        now: Instant,
    ) -> u64 {
        let id = self.reserve();
        self.attach(id, label, cancel, handle, now);
        id
    }

    /// Reserve the next id (before a worker/handle exists).
    pub(crate) fn reserve(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Attach a spawned worker to a previously reserved id.
    pub(crate) fn attach(
        &mut self,
        id: u64,
        label: String,
        cancel: Arc<AtomicBool>,
        handle: JoinHandle<()>,
        now: Instant,
    ) {
        self.running.push(Task {
            id,
            label,
            cancel,
            handle,
            started: now,
        });
    }

    /// Drop tasks whose worker thread has exited.
    pub(crate) fn reap(&mut self) {
        self.running.retain(|t| !t.handle.is_finished());
    }

    /// Trip task `id`'s cancel flag; `false` if no such task.
    pub(crate) fn cancel(&self, id: u64) -> bool {
        match self.running.iter().find(|t| t.id == id) {
            Some(t) => {
                t.cancel.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    /// Trip every running task's cancel flag; returns how many.
    pub(crate) fn cancel_all(&self) -> usize {
        for t in &self.running {
            t.cancel.store(true, Ordering::Relaxed);
        }
        self.running.len()
    }

    pub(crate) fn len(&self) -> usize {
        self.running.len()
    }

    /// One line per running task: `#<id> · <label> · <age>`.
    pub(crate) fn describe(&self, now: Instant) -> Vec<String> {
        self.running
            .iter()
            .map(|t| {
                let secs = now.saturating_duration_since(t.started).as_secs();
                let age = if secs >= 60 {
                    format!("{}m", secs / 60)
                } else {
                    format!("{secs}s")
                };
                format!("#{} \u{00b7} {} \u{00b7} {age}", t.id, t.label)
            })
            .collect()
    }

    /// Join all worker threads (called on stdin EOF so output isn't truncated).
    pub(crate) fn join_all(&mut self) {
        for t in self.running.drain(..) {
            let _ = t.handle.join();
        }
    }
}

#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tests;
```

> The test module above is written inline in Step 1 for readability; move it to a sibling `crates/crew-plugin/src/broker/tasks_tests.rs` and keep the `#[cfg(test)] #[path = "tasks_tests.rs"] mod tests;` line at the bottom of `tasks.rs` (matches the repo's split-test convention). If you keep it in-file instead, drop the `#[path]` line — either is fine, just don't have both.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-plugin tasks 2>&1 | tail -5`
Expected: all Task-1 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/tasks.rs crates/crew-plugin/src/broker/tasks_tests.rs crates/crew-plugin/src/broker/mod.rs
git commit -m "feat(crew-plugin): background task registry (ids, per-task cancel, cap)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: per-task cancel flag on `Session`

**Files:**
- Modify: `crates/crew-plugin/src/broker/session.rs` (the `Session` struct, `Default`, `snapshot`, `running`, and the `snapshot_shares_the_cancel_flag` test)

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `Session::snapshot_with_cancel(&self, cancel: Arc<AtomicBool>) -> Session` — a worker copy that uses the GIVEN cancel flag (so the registry can trip it), sharing turns/tokens/mcp/plan as before.
  - The `busy` field and `running()` method are REMOVED (the registry replaces them). `cancel`/`cancelled()` stay (now per-task on snapshots).

- [ ] **Step 1: Write the failing test**

Replace the existing `snapshot_shares_the_cancel_flag` test in session.rs's `tests` module with:

```rust
    #[test]
    fn snapshot_with_cancel_uses_the_given_flag() {
        let s = Session::new();
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let snap = s.snapshot_with_cancel(std::sync::Arc::clone(&flag));
        // Tripping the registry-held flag cancels the snapshot's broker/loop.
        flag.store(true, Ordering::Relaxed);
        assert!(snap.cancelled(), "snapshot observes its own task's cancel flag");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-plugin snapshot_with_cancel 2>&1 | tail -5`
Expected: FAIL — `snapshot_with_cancel` not defined.

- [ ] **Step 3: Implement**

In `session.rs`:
1. Remove the `pub busy: Arc<Mutex<Option<String>>>` field from `Session`, its `Default` init, and its `snapshot` copy. Remove the `pub fn running(&self)` method.
2. Replace `snapshot` with `snapshot_with_cancel`:

```rust
    /// A worker-thread copy for one task: its own override map (reads only),
    /// the SAME shared counters (turns/tokens) and MCP/plan, but the caller's
    /// per-task `cancel` flag so `/stop #N` reaches exactly this task.
    pub fn snapshot_with_cancel(&self, cancel: Arc<AtomicBool>) -> Self {
        Self {
            overrides: self.overrides.clone(),
            cancel,
            turns: Arc::clone(&self.turns),
            tokens: Arc::clone(&self.tokens),
            mcp: Arc::clone(&self.mcp),
            plan: Arc::clone(&self.plan),
        }
    }
```
3. Keep the `cancel: Arc<AtomicBool>` field (the main Session's is unused now but harmless — default it fresh), `cancelled()`, and `broker()` (which uses `self.cancel`, i.e. the per-task flag on a snapshot). Remove the now-unused `Mutex` import if `busy` was its only user (check: `mcp` uses `Mutex`, so keep it).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-plugin session 2>&1 | tail -5`
Expected: PASS. (Build will still fail in `stdio.rs` — Task 3 fixes the callers. If you need a green build to run session tests, temporarily `cargo test -p crew-plugin --lib session` may still fail to compile due to stdio; in that case do Task 3 before running — note this and proceed.)

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/session.rs
git commit -m "refactor(crew-plugin): per-task cancel flag via snapshot_with_cancel

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
(If the crate doesn't compile standalone here because `stdio.rs` still references the old API, combine this commit with Task 3 — commit once at the end of Task 3. Note which you did.)

---

### Task 3: concurrent spawn + `/stop [#N]` + `/tasks` in `stdio.rs`

**Files:**
- Modify: `crates/crew-plugin/src/broker/stdio.rs` (`run_broker_stdio` loop and `send`)
- Test: `crates/crew-plugin/tests/` (an integration test driving the broker) OR an in-file `#[cfg(test)]` on stdio if feasible — see Step 1.

**Interfaces:**
- Consumes: Task 1 `tasks::Tasks`; Task 2 `Session::snapshot_with_cancel`.

- [ ] **Step 1: Write the failing test**

There is an existing e2e harness at `crates/crew-plugin/tests/` that feeds the broker JSON commands and parses events (see `tests/e2e_relay.rs`/`tests/common/mod.rs`). Add a test there using `CREW_BROKER_MOCK_REPLY` so no API keys are needed. Model it on the existing e2e tests' helpers (spawn the broker binary or call `run_broker_stdio` over piped stdio — reuse whatever `tests/common/mod.rs` exposes). Assertions:
- Sending two task messages back-to-back both produce a `▸ task #1 started` and `▸ task #2 started` message (not a "busy" rejection).
- `/tasks` lists both (`#1`, `#2`).
- `/stop #1` emits a stop acknowledgement naming `#1`; bare `/stop` acknowledges cancelling all.
- A task's streamed messages carry `meta` starting with `task:`.

If the existing harness can't easily assert interleaving, at minimum assert: a second Send while one runs is NOT rejected (no "busy" text) and both get a "task #" start line. Match the harness's actual helper API — read `tests/common/mod.rs` first and use its functions verbatim.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-plugin --test <name> 2>&1 | tail -8`
Expected: FAIL — the second Send is rejected as "busy" (old behavior) / `/tasks` unknown.

- [ ] **Step 3: Implement**

Rewrite `run_broker_stdio` and `send` (in stdio.rs) to use the registry. Replace `let mut worker: Option<JoinHandle<()>> = None;` with `let mut tasks = super::tasks::Tasks::new();`, thread `&mut tasks` through `send`, and on EOF call `tasks.join_all()`.

New `send` (replaces the current one):

```rust
/// Route one Send. `/stop [#N]` and `/tasks` and quick constructs answer
/// inline; every other task spawns a NEW background worker (up to the cap),
/// so several run at once.
fn send(
    text: String,
    out: &Out,
    session: &mut Session,
    tasks: &mut super::tasks::Tasks,
) -> anyhow::Result<()> {
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;
    tasks.reap();
    let trimmed = text.trim().to_string();

    // /stop [#N] — cancel one task or all.
    if trimmed == "/stop" || trimmed.starts_with("/stop ") {
        let arg = trimmed.strip_prefix("/stop").unwrap().trim();
        if arg.is_empty() {
            let n = tasks.cancel_all();
            let m = if n == 0 { "nothing is running".into() } else { format!("stopping all {n} task(s)\u{2026}") };
            return emit(out, &msg("crew", m));
        }
        let id: Option<u64> = arg.trim_start_matches('#').parse().ok();
        let m = match id {
            Some(id) if tasks.cancel(id) => format!("stopping task #{id}\u{2026}"),
            Some(id) => format!("no task #{id}"),
            None => format!("usage: /stop [#id]"),
        };
        return emit(out, &msg("crew", m));
    }

    // /tasks — list running tasks.
    if trimmed == "/tasks" {
        let lines = tasks.describe(Instant::now());
        let body = if lines.is_empty() { "no background tasks running".into() } else { lines.join("\n") };
        return emit(out, &msg("crew", body));
    }

    if super::commands::is_quick(&trimmed) {
        return super::commands::handle(session, &trimmed, &mut |ev| emit(out, &ev));
    }

    if !tasks.admit() {
        return emit(out, &msg("crew", format!("at capacity ({} tasks) \u{2014} /stop one first", tasks.len())));
    }

    // The worker closure needs the task id (to stamp `meta` and print the
    // start/done lines), but `attach` needs the JoinHandle which only exists
    // after `spawn` — so reserve the id first, spawn, then attach.
    session.turns.fetch_add(1, Ordering::Relaxed);
    let label: String = trimmed.chars().take(40).collect();
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let mut snap = session.snapshot_with_cancel(std::sync::Arc::clone(&cancel));
    let out_thread = Arc::clone(out);
    let is_cmd = super::commands::is_command(&trimmed);
    let id = tasks.reserve();
    emit(out, &msg("crew", format!("\u{25b8} task #{id} started \u{00b7} {label}")))?;
    let handle = std::thread::spawn(move || {
        let tokens = Arc::clone(&snap.tokens);
        // Stamp every Message event with this task's id, and count Stats.
        let mut counting = |mut ev: PluginEvent| {
            if let PluginEvent::Stats { tokens: t, .. } = &ev {
                tokens.fetch_add(*t, Ordering::Relaxed);
            }
            if let PluginEvent::Message { meta, .. } = &mut ev {
                if meta.is_empty() { *meta = format!("task:{id}"); }
            }
            emit(&out_thread, &ev)
        };
        let res = if is_cmd {
            super::commands::handle(&mut snap, &trimmed, &mut counting)
        } else {
            relay_counting(&trimmed, &snap, &mut counting)
        };
        let done = match (res, snap.cancelled()) {
            (Err(e), _) => format!("\u{2717} task #{id}: {e}"),
            (Ok(_), true) => format!("\u{2717} task #{id} stopped"),
            (Ok(_), false) => format!("\u{2713} task #{id} done"),
        };
        let _ = emit(&out_thread, &msg("crew", done));
    });
    tasks.attach(id, label, cancel, handle, Instant::now());
    Ok(())
```
Update the `run_broker_stdio` match arm: `PluginCommand::Send { text, .. } => send(text, &out, &mut session, &mut tasks)?`. On loop exit, replace the single-handle join with `tasks.join_all();`. Remove the old `busy`-based rejection entirely.

> `msg(...)` events default `meta` to `""`; the start/done "crew" lines are NOT stamped (they're pane-level, not task-tagged) — the `if meta.is_empty()` guard only stamps the relay's own messages. That's intended.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-plugin 2>&1 | tail -5` — all pass (incl. the new e2e).
Run: `cargo clippy -p crew-plugin -- -D warnings 2>&1 | grep -E "^warning|^error" || echo CLEAN`

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/stdio.rs crates/crew-plugin/src/broker/tasks.rs crates/crew-plugin/src/broker/tasks_tests.rs crates/crew-plugin/tests/
git commit -m "feat(crew-plugin): run background tasks concurrently with /stop [#N] and /tasks

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: crew-app — `#N` task chip on tagged messages

**Files:**
- Modify: `crates/crew-app/src/chatmsgs.rs` (or wherever a message row is built from its `sender`/`meta`) — render a dim `#N` chip for a message whose meta is `task:<id>`.
- Test: the same file's test module.

**Interfaces:**
- Consumes: the `Message { meta }` carrying `task:<id>` (Task 3). Find where crew-app stores a message's meta — check `chatlayout::Message` (it has `meta` per the chip-grid work) and how `chatmsgs` renders a card header.

- [ ] **Step 1: Write the failing test**

First locate the message-card header renderer: `grep -rn "meta" crates/crew-app/src/chatmsgs.rs crates/crew-app/src/chatlayout.rs`. Add a pure helper `task_tag(meta: &str) -> Option<u64>` that parses `"task:<id>"`, and a test:

```rust
#[test]
fn task_tag_parses_the_meta() {
    assert_eq!(super::task_tag("task:3"), Some(3));
    assert_eq!(super::task_tag(""), None);
    assert_eq!(super::task_tag("other"), None);
}
```
Plus a render-level test (mirroring existing chatmsgs tests): a message with `meta="task:2"` produces a cell run containing `#2` in the header, dim-colored.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app task_tag 2>&1 | tail -5`
Expected: FAIL — `task_tag` not defined.

- [ ] **Step 3: Implement**

Add:
```rust
/// The task id carried in a message's `meta` (`"task:<id>"`), if any.
pub(crate) fn task_tag(meta: &str) -> Option<u64> {
    meta.strip_prefix("task:").and_then(|s| s.parse().ok())
}
```
In the message-card header builder, when `task_tag(&msg.meta)` is `Some(id)`, prepend a dim `#<id> ` chip (color `crew_theme::theme().text_muted`) before the sender. Follow the existing header cell-placement pattern in `chatmsgs.rs` (use the same `push`/`place_row` helper the file already uses).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-app 2>&1 | tail -3` — all pass.
Run: `cargo clippy -p crew-app -- -D warnings 2>&1 | grep -E "^warning|^error" || echo CLEAN`
Run: `cargo test --workspace 2>&1 | grep -E "test result: FAILED" || echo WORKSPACE-GREEN`

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatmsgs.rs
git commit -m "feat(crew-app): tag background-task messages with a #N chip

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
