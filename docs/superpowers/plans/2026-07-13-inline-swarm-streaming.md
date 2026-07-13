# Inline Swarm Streaming + Universal Pane Close — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stream `/crew` swarm progress live inside the chat pane (no companion hive pane), and give every pane a `[x]` close button next to `[-]` plus Esc-to-close on swarm panes.

**Architecture:** The broker's `run_with` emits Hive/chat events *while* the scheduler runs (drain future calls `emit` directly inside `tokio::join!`). On the app side, `PluginEvent::HivePlan`/`Hive` stop being host actions and become ChatPane state (`SwarmStatus`), rendered as a live status block at the bottom of the message area and folded into a transcript message when the run ends. The border-button pattern (`panecard.rs` draw + `hit.rs` rect + `events.rs` click) is extended from `[-]` to `[-][x]`.

**Tech Stack:** Rust, tokio (current-thread runtime in broker worker), winit, existing crew-render CellView pipeline.

**Spec:** `docs/superpowers/specs/2026-07-13-inline-swarm-streaming-design.md`

## Global Constraints

- Branch: `feat/inline-swarm-streaming` (already created; spec committed).
- Pre-commit hook runs `cargo fmt --check` and `cargo check` — run `cargo fmt` before every commit.
- All work is synchronous on the winit thread in crew-app: no blocking I/O, no subprocess spawns in render/absorb paths.
- No title bars: buttons ride the fieldset border (existing `[-]` pattern).
- Protocol (`crew-plugin/src/protocol.rs`) is UNCHANGED — `HivePlan`/`Hive` stay on the wire.
- Test commands: `cargo test -p crew-plugin`, `cargo test -p crew-app` (crew-hive untouched).
- Commit messages end with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

---

### Task 1: Broker live event drain

**Files:**
- Modify: `crates/crew-plugin/src/broker/swarm.rs:161-204` (the execute + drain section of `run_with`) and its `tests` module.

**Interfaces:**
- Consumes: existing `translate(&HiveEvent, &titles, &mut agent_task) -> Vec<PluginEvent>` (unchanged).
- Produces: `run_with` with identical signature and event ordering (HivePlan first, aggregate Stats before final summary) but events now emitted during execution. Later tasks rely only on the ordering, not on code here.

- [ ] **Step 1: Write the failing liveness test**

Add to the `tests` module in `crates/crew-plugin/src/broker/swarm.rs`. The factory snapshots how many events had been emitted when each task started; the merge task (the one with deps) must start *after* some leaf events were already emitted:

```rust
    // Live drain: events must reach `emit` WHILE the run executes, not be
    // buffered until the scheduler finishes. The merge task starts only
    // after both leaves complete — by then their events must have been
    // emitted already.
    #[test]
    fn events_are_emitted_during_the_run_not_after() {
        use crew_hive::agent::{Agent, AgentContext};
        use crew_hive::board::TaskResult;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;

        struct SnoopAgent {
            emitted: Arc<AtomicUsize>,
            merge_snapshot: Arc<Mutex<Option<usize>>>,
        }
        impl Agent for SnoopAgent {
            fn run(
                &self,
                ctx: AgentContext,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = TaskResult> + Send>>
            {
                let emitted = Arc::clone(&self.emitted);
                let snap = Arc::clone(&self.merge_snapshot);
                Box::pin(async move {
                    if !ctx.deps.is_empty() {
                        // The merge task: record how many events the host had
                        // received by the time it started.
                        *snap.lock().unwrap() = Some(emitted.load(Ordering::SeqCst));
                    }
                    let output = format!("snoop:{}", ctx.task.id.0);
                    ctx.bus.publish(crew_hive::HiveEvent::OutputChunk {
                        agent: ctx.agent.clone(),
                        text: output.clone(),
                    });
                    TaskResult {
                        task: ctx.task.id,
                        output,
                        success: true,
                    }
                })
            }
        }
        struct SnoopFactory {
            emitted: Arc<AtomicUsize>,
            merge_snapshot: Arc<Mutex<Option<usize>>>,
        }
        impl crew_hive::AgentFactory for SnoopFactory {
            fn make(&self, _kind: &crew_hive::AgentKind) -> Box<dyn Agent> {
                Box::new(SnoopAgent {
                    emitted: Arc::clone(&self.emitted),
                    merge_snapshot: Arc::clone(&self.merge_snapshot),
                })
            }
        }

        let emitted = Arc::new(AtomicUsize::new(0));
        let merge_snapshot = Arc::new(Mutex::new(None));
        let counter = Arc::clone(&emitted);
        run_with(
            "build the thing",
            Arc::new(StubPlanner { fanout: 2 }),
            Arc::new(SnoopFactory {
                emitted,
                merge_snapshot: Arc::clone(&merge_snapshot),
            }),
            None,
            Arc::new(AtomicBool::new(false)),
            &mut |_ev| {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap();
        let snap = merge_snapshot
            .lock()
            .unwrap()
            .expect("merge task must have run");
        assert!(
            snap > 2, // more than just HivePlan + plan-summary message
            "leaf events must be emitted before the merge task starts (got {snap})"
        );
    }
```

Check the exact import paths compile (`crew_hive::agent::{Agent, AgentContext}`, `crew_hive::board::TaskResult` — adjust to the crate's re-exports, e.g. `crew_hive::TaskResult`, if the module paths are private).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-plugin events_are_emitted_during_the_run_not_after`
Expected: FAIL — with the post-hoc drain the snapshot is ≤ 2 (only `HivePlan` + the plan-summary `Message` are emitted before execution).

- [ ] **Step 3: Restructure `run_with` to emit inside the drain**

Replace the block from the `let (tx, rx) = std::sync::mpsc::channel…` line through the `while let Ok(ev) = rx.try_recv() { … }` loop (currently swarm.rs:170-204) with:

```rust
    // Drain the bus and emit LIVE while the scheduler runs — join! interleaves
    // the three futures on this current-thread runtime, so each event reaches
    // the host as it happens instead of after the run (frozen-looking runs).
    let mut agent_task: HashMap<u64, TaskId> = HashMap::new();
    let mut tokens_total: u64 = 0;
    let mut emit_err: Option<anyhow::Error> = None;
    let outcome = rt.block_on(async {
        let drain = async {
            loop {
                match sub.recv().await {
                    Ok(ev) => {
                        if emit_err.is_some() {
                            continue; // keep consuming so the scheduler finishes
                        }
                        if let HiveEvent::TokenDelta { input, output, .. } = &ev {
                            tokens_total += u64::from(*input) + u64::from(*output);
                        }
                        let r = emit(PluginEvent::Hive { event: ev.clone() }).and_then(|()| {
                            for out in translate(&ev, &titles, &mut agent_task) {
                                emit(out)?;
                            }
                            Ok(())
                        });
                        if let Err(e) = r {
                            emit_err = Some(e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        match governor {
            Some(g) => tokio::join!(sched.run(), drain, g).0,
            None => tokio::join!(sched.run(), drain).0,
        }
    });
    if let Some(e) = emit_err {
        return Err(e);
    }
```

Delete the now-dead comment block above it ("Translate the drained telemetry (the run has completed…)"). The `let mut agent_task` / `tokens_total` declarations that used to sit below `block_on` are replaced by the ones above. Everything from the sink-gathering (`let sink_ids`) onward is unchanged. Also update the module doc comment (line 2): the events now stream live.

Note on borrows: only the `drain` future captures `emit`, `tokens_total`, `agent_task`, `emit_err`; `sched.run()` and the governor don't touch them, so the single mutable borrow is fine, and `join!` on a current-thread runtime needs no `Send`.

- [ ] **Step 4: Run the broker test suite**

Run: `cargo test -p crew-plugin`
Expected: ALL PASS — the new liveness test plus the existing ordering tests (`plain_task_emits_plan_then_hive_events_then_summary`, `run_emits_an_aggregate_stats_event_with_tokens_and_exchange_count`, `task_failure_becomes_a_chat_message_not_a_connection_error`, `pre_cancelled_run_reports_cancellation`).

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/swarm.rs
git commit -m "fix(broker): stream swarm telemetry live during the run

The drain future now emits Hive + chat events inside the scheduler
join! instead of buffering everything until the run completes, so
/crew swarm runs no longer look frozen.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: PtyTerm kills its child on drop

**Files:**
- Modify: `crates/crew-term/src/pty.rs` (struct `PtyTerm`, field `_child`).

**Interfaces:**
- Produces: no API change — `PtyTerm` drop now explicitly kills the child instead of relying on SIGHUP from master-close. Task 7's `[x]` click depends on this teardown being reliable.

- [ ] **Step 1: Rename `_child` to `child` and add the Drop impl**

In `crates/crew-term/src/pty.rs`, rename the field `_child: Box<dyn portable_pty::Child + Send + Sync>` to `child` (fix the constructor(s) that set it), and add at the end of the impl blocks:

```rust
impl Drop for PtyTerm {
    /// Kill the child explicitly — dropping the master only HUPs the child
    /// the next time it touches the tty, which leaves quiet processes
    /// running after the pane is closed with the [x] border button.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
```

- [ ] **Step 2: Build and test the crate**

Run: `cargo test -p crew-term`
Expected: PASS (existing PTY tests spawn real shells; if any test hangs on `wait()`, gate the `wait` behind `kill` success: `if self.child.kill().is_ok() { let _ = self.child.wait(); }`).

- [ ] **Step 3: Commit**

```bash
cargo fmt && git add crates/crew-term/src/pty.rs
git commit -m "fix(term): kill the PTY child on drop

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: ChatPane swarm state (`SwarmStatus`)

**Files:**
- Create: `crates/crew-app/src/chatswarm.rs`
- Create: `crates/crew-app/src/chatswarm_tests.rs`
- Modify: `crates/crew-app/src/chat.rs` (add `swarm` field, `is_busy`, `Error` arm)
- Modify: `crates/crew-app/src/main.rs` (register `mod chatswarm;` alongside the other `chat*` modules)

**Interfaces:**
- Consumes: `crew_hive::{HiveEvent, TaskId, TaskSpec, TaskState}` (TaskState variants: `Pending, Ready, Running, Done, Failed, Cancelled`); `crate::chatlayout::Message`.
- Produces (Tasks 4–5 rely on these exact names):
  - `pub(crate) struct SwarmStatus { pub tasks: Vec<SwarmTask>, .. }`
  - `pub(crate) struct SwarmTask { pub id: crew_hive::TaskId, pub title: String, pub state: crew_hive::TaskState, pub tokens: u64 }`
  - `SwarmStatus::new(tasks: Vec<crew_hive::TaskSpec>) -> SwarmStatus`
  - `SwarmStatus::apply(&mut self, ev: &crew_hive::HiveEvent)`
  - `SwarmStatus::finished(&self) -> bool`
  - `SwarmStatus::record_text(&self) -> String`
  - `ChatPane::absorb_hive_plan(&mut self, tasks: Vec<crew_hive::TaskSpec>)`
  - `ChatPane::absorb_hive(&mut self, ev: &crew_hive::HiveEvent)`
  - `ChatPane::fold_swarm(&mut self)`
  - `ChatPane` field: `pub(crate) swarm: Option<crate::chatswarm::SwarmStatus>`

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/chatswarm_tests.rs`:

```rust
use super::*;
use crate::chat::ChatPane;
use crew_hive::{AgentId, AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn spec(id: u64, title: &str) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
    }
}

fn pane() -> ChatPane {
    ChatPane::new(Plugin::disconnected(), "crew".into())
}

#[test]
fn hive_plan_builds_pending_tasks() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research"), spec(1, "merge")]);
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(s.tasks.len(), 2);
    assert!(s.tasks.iter().all(|t| t.state == TaskState::Pending));
    assert!(!s.finished());
}

#[test]
fn agent_spawned_marks_running_and_token_deltas_accumulate_via_agent_map() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research")]);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: AgentId(7),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: AgentId(7),
        input: 100,
        output: 50,
    });
    let s = p.swarm.as_ref().unwrap();
    assert_eq!(s.tasks[0].state, TaskState::Running);
    assert_eq!(s.tasks[0].tokens, 150);
}

#[test]
fn run_completion_folds_the_block_into_a_transcript_message() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "research"), spec(1, "merge")]);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    assert!(p.swarm.is_some(), "one task still pending — not folded yet");
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(1),
        state: TaskState::Failed,
    });
    // All terminal: state cleared, record message pushed.
    assert!(p.swarm.is_none());
    let last = p.messages.last().unwrap();
    assert_eq!(last.sender, "crew");
    assert!(last.text.contains("✓ research"));
    assert!(last.text.contains("✗ merge"));
}

#[test]
fn a_second_plan_resets_the_block() {
    let mut p = pane();
    p.absorb_hive_plan(vec![spec(0, "a")]);
    p.absorb_hive_plan(vec![spec(0, "x"), spec(1, "y")]);
    assert_eq!(p.swarm.as_ref().unwrap().tasks.len(), 2);
}

#[test]
fn events_without_a_plan_are_ignored() {
    let mut p = pane();
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    }); // must not panic
    assert!(p.swarm.is_none());
}

#[test]
fn swarm_in_flight_keeps_the_pane_busy() {
    let mut p = pane();
    assert!(!p.is_busy());
    p.absorb_hive_plan(vec![spec(0, "a")]);
    assert!(p.is_busy());
}
```

If `Plugin::disconnected()` doesn't exist, use whatever constructor `chat_tests.rs` already uses to build a ChatPane without a live broker — copy its helper.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatswarm`
Expected: FAIL to compile — `chatswarm` module and `swarm` field don't exist.

- [ ] **Step 3: Implement `chatswarm.rs` and wire the field**

Create `crates/crew-app/src/chatswarm.rs`:

```rust
//! Live swarm-run status for the chat pane: `HivePlan` opens a task-list
//! block, `Hive` telemetry updates it, and when every task reaches a terminal
//! state the block folds into a transcript message — the durable record of
//! the run. Rendering lives in `chatswarmview`.
use std::collections::HashMap;

use crew_hive::{HiveEvent, TaskId, TaskSpec, TaskState};

use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// One planned task's live state in the block.
pub(crate) struct SwarmTask {
    pub id: TaskId,
    pub title: String,
    pub state: TaskState,
    /// Tokens spent by the agent running this task (input + output).
    pub tokens: u64,
}

/// The whole run's live state, built from `HivePlan` and fed by `Hive` events.
pub(crate) struct SwarmStatus {
    pub tasks: Vec<SwarmTask>,
    /// agent id → task id (from `AgentSpawned`) — `TokenDelta` only names agents.
    agent_task: HashMap<u64, TaskId>,
}

impl SwarmStatus {
    pub(crate) fn new(tasks: Vec<TaskSpec>) -> Self {
        SwarmStatus {
            tasks: tasks
                .into_iter()
                .map(|t| SwarmTask {
                    id: t.id,
                    title: t.title,
                    state: TaskState::Pending,
                    tokens: 0,
                })
                .collect(),
            agent_task: HashMap::new(),
        }
    }

    fn task_mut(&mut self, id: TaskId) -> Option<&mut SwarmTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub(crate) fn apply(&mut self, ev: &HiveEvent) {
        match ev {
            HiveEvent::AgentSpawned { agent, task } => {
                self.agent_task.insert(agent.0, *task);
                if let Some(t) = self.task_mut(*task) {
                    t.state = TaskState::Running;
                }
            }
            HiveEvent::TaskStateChanged { task, state } => {
                if let Some(t) = self.task_mut(*task) {
                    t.state = state.clone();
                }
            }
            HiveEvent::TokenDelta {
                agent,
                input,
                output,
            } => {
                if let Some(&task) = self.agent_task.get(&agent.0) {
                    if let Some(t) = self.task_mut(task) {
                        t.tokens += u64::from(*input) + u64::from(*output);
                    }
                }
            }
            // Failed also arrives as TaskStateChanged(Failed); chunks/cost
            // land in the transcript via the broker's Message translation.
            HiveEvent::OutputChunk { .. }
            | HiveEvent::CostDelta { .. }
            | HiveEvent::Failed { .. } => {}
        }
    }

    /// Every task reached a terminal state.
    pub(crate) fn finished(&self) -> bool {
        self.tasks.iter().all(|t| {
            matches!(
                t.state,
                TaskState::Done | TaskState::Failed | TaskState::Cancelled
            )
        })
    }

    /// The block as a markdown list — the transcript record on fold.
    pub(crate) fn record_text(&self) -> String {
        self.tasks
            .iter()
            .map(|t| {
                let glyph = glyph(&t.state);
                if t.tokens > 0 {
                    format!("- {glyph} {} — {} tok", t.title, t.tokens)
                } else {
                    format!("- {glyph} {}", t.title)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The state glyph shared by the live block and the folded record.
pub(crate) fn glyph(state: &TaskState) -> char {
    match state {
        TaskState::Pending | TaskState::Ready => '·',
        TaskState::Running => '⠿', // live view animates; record shows a static mark
        TaskState::Done => '✓',
        TaskState::Failed => '✗',
        TaskState::Cancelled => '⊘',
    }
}

impl ChatPane {
    /// A swarm plan landed: open (or reset) the live block.
    pub(crate) fn absorb_hive_plan(&mut self, tasks: Vec<TaskSpec>) {
        self.swarm = Some(SwarmStatus::new(tasks));
    }

    /// Forwarded telemetry; folds the block once the run is over.
    pub(crate) fn absorb_hive(&mut self, ev: &HiveEvent) {
        let Some(s) = self.swarm.as_mut() else { return };
        s.apply(ev);
        if s.finished() {
            self.fold_swarm();
        }
    }

    /// Retire the live block into a transcript message (the run's record).
    /// Also called on broker `Error` so a dead run leaves its partial state
    /// in the transcript instead of a forever-frozen block.
    pub(crate) fn fold_swarm(&mut self) {
        let Some(s) = self.swarm.take() else { return };
        if self.scroll > 0 {
            self.unread += 1;
        }
        self.messages.push(Message {
            sender: "crew".into(),
            text: s.record_text(),
            ts: String::new(),
            meta: String::new(),
        });
    }
}

#[cfg(test)]
#[path = "chatswarm_tests.rs"]
mod tests;
```

In `crates/crew-app/src/chat.rs`:
- add the field to the struct (after `tick_open`):

```rust
    /// The live /crew swarm-run block (from `HivePlan`/`Hive` events); folded
    /// into a transcript message when the run ends (see `chatswarm`).
    pub(crate) swarm: Option<crate::chatswarm::SwarmStatus>,
```

- add `swarm: None,` in `ChatPane::new`.
- extend `is_busy`:

```rust
    pub fn is_busy(&self) -> bool {
        self.awaiting || !self.active.is_empty() || self.swarm.is_some()
    }
```

- in `poll()`'s `PluginEvent::Error { .. }` arm, add `self.fold_swarm();` before `self.connected = false;`.

In `crates/crew-app/src/main.rs`, add `mod chatswarm;` in the module list (alphabetical with the other `chat*` mods).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app chatswarm`
Expected: ALL PASS. Also run `cargo test -p crew-app chat` — existing chat tests stay green.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-app/src/chatswarm.rs crates/crew-app/src/chatswarm_tests.rs crates/crew-app/src/chat.rs crates/crew-app/src/main.rs
git commit -m "feat(chat): SwarmStatus — pane-local live swarm-run state

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: Reroute HivePlan/Hive to the pane; delete the companion hive pane

**Files:**
- Modify: `crates/crew-app/src/chatevents.rs` (drop the two `HostAction` variants + classify arms)
- Modify: `crates/crew-app/src/chat.rs` (`poll()` absorbs the two events)
- Modify: `crates/crew-app/src/poll.rs:263-264` (remove the two action arms)
- Modify: `crates/crew-app/src/app.rs:103-124` (`close_pane` loses the companion special case)
- Delete: `crates/crew-app/src/hivepane.rs`; remove `mod hivepane;` from `main.rs`
- Modify: `crates/crew-app/src/swarmpane.rs` (delete `SwarmPane::for_remote` + `apply_remote`), `crates/crew-app/src/swarm/tests.rs` (delete their tests)
- Modify: `crates/crew-app/src/chat_tests.rs`, `crates/crew-app/src/app_tests.rs` (update classify/hive expectations)

**Interfaces:**
- Consumes: `ChatPane::absorb_hive_plan` / `absorb_hive` from Task 3.
- Produces: `HostAction` with exactly two variants (`SpawnPane`, `SendPane`); `close_pane` body is the plain remove + reconcile. Task 7 relies on `close_pane(idx)` unchanged in signature.

- [ ] **Step 1: Write/adjust the failing test**

In `crates/crew-app/src/chat_tests.rs` (or wherever `classify` is tested — search `rg -n "HivePlan" crates/crew-app/src/chat_tests.rs`), replace any test asserting `classify(HivePlan) == Some(HostAction::HivePlan…)` with:

```rust
#[test]
fn hive_events_are_pane_state_not_host_actions() {
    use crew_plugin::PluginEvent;
    assert!(crate::chat::classify(&PluginEvent::HivePlan { tasks: vec![] }).is_none());
    assert!(crate::chat::classify(&PluginEvent::Hive {
        event: crew_hive::HiveEvent::TaskStateChanged {
            task: crew_hive::TaskId(0),
            state: crew_hive::TaskState::Running,
        }
    })
    .is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app hive_events_are_pane_state`
Expected: FAIL — classify still returns `Some`.

- [ ] **Step 3: Implement the reroute + deletions**

1. `chatevents.rs`: delete the `HivePlan`/`Hive` variants from `HostAction` and their two arms in `classify`.
2. `chat.rs` `poll()`: add to the inner `match ev` (after the `Message` arm):

```rust
                    PluginEvent::HivePlan { tasks } => self.absorb_hive_plan(tasks),
                    PluginEvent::Hive { event } => self.absorb_hive(&event),
```

3. `poll.rs`: delete the `HostAction::HivePlan`/`HostAction::Hive` arms.
4. `app.rs` `close_pane`: replace lines 105-124 (companion lookup + dual-remove) with:

```rust
            self.panes.remove(idx);
            self.grid.on_close(idx);
```

5. Delete `crates/crew-app/src/hivepane.rs` and its `mod hivepane;` line in `main.rs`.
6. `swarmpane.rs`: delete `for_remote` and `apply_remote` (verify no callers remain: `rg -n "for_remote|apply_remote" crates/`) and delete their tests in `swarm/tests.rs`.
7. Fix any `app_tests.rs` tests that call `hive_plan`/`hive_event` — delete them (their behavior is now covered by `chatswarm_tests.rs`).

- [ ] **Step 4: Run the full app test suite**

Run: `cargo test -p crew-app`
Expected: ALL PASS, no remaining references (`rg -n "hive_plan|hive_event|HostAction::Hive" crates/crew-app/src` returns nothing).

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add -A crates/crew-app/src
git commit -m "feat(chat): swarm telemetry feeds the chat pane; drop the companion hive pane

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: Render the live status block in the chat pane

**Files:**
- Create: `crates/crew-app/src/chatswarmview.rs`
- Create: `crates/crew-app/src/chatswarmview_tests.rs`
- Modify: `crates/crew-app/src/chatplace.rs:82-86` (`msg_rows_budget` subtracts the block rows)
- Modify: `crates/crew-app/src/chatview.rs` (`cells` draws the block under the message area)
- Modify: `crates/crew-app/src/main.rs` (register `mod chatswarmview;`)

**Interfaces:**
- Consumes: `pane.swarm: Option<SwarmStatus>` and `chatswarm::glyph` from Task 3; `crate::update::SPINNER`; `crate::anim::now_ms()`; `crate::palette::accent()`; `crew_theme::theme()` colors (`text_muted`, `activity`, `bell`).
- Produces:
  - `pub(crate) fn swarm_rows(pane: &ChatPane, rows: u16) -> u16` — rows the live block occupies (0 when no run).
  - `pub(crate) fn block_cells(pane: &ChatPane, cols: u16, top_row: u16, now_ms: u64) -> Vec<CellView>` — the block drawn starting at `top_row`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/chatswarmview_tests.rs`:

```rust
use super::*;
use crate::chat::ChatPane;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn pane_with_swarm(n: u64) -> ChatPane {
    let mut p = ChatPane::new(Plugin::disconnected(), "crew".into());
    let tasks = (0..n)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: format!("task-{i}"),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
        })
        .collect();
    p.absorb_hive_plan(tasks);
    p
}

#[test]
fn no_swarm_no_rows() {
    let p = ChatPane::new(Plugin::disconnected(), "crew".into());
    assert_eq!(swarm_rows(&p, 40), 0);
    assert!(block_cells(&p, 80, 5, 0).is_empty());
}

#[test]
fn one_row_per_task_capped_at_eight() {
    assert_eq!(swarm_rows(&pane_with_swarm(3), 40), 3);
    assert_eq!(swarm_rows(&pane_with_swarm(20), 40), 8);
}

#[test]
fn block_rows_render_titles_with_state_glyphs() {
    let mut p = pane_with_swarm(2);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    let cells = block_cells(&p, 80, 10, 0);
    let row10: String = cells.iter().filter(|c| c.row == 10).map(|c| c.c).collect();
    let row11: String = cells.iter().filter(|c| c.row == 11).map(|c| c.c).collect();
    assert!(row10.contains('✓') && row10.contains("task-0"), "{row10}");
    assert!(row11.contains("task-1"), "{row11}");
}

#[test]
fn token_counts_right_aligned_on_wide_panes_dropped_on_narrow() {
    let mut p = pane_with_swarm(1);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: crew_hive::AgentId(1),
        input: 12_000,
        output: 400,
    });
    let wide: String = block_cells(&p, 60, 0, 0)
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.c)
        .collect();
    assert!(wide.contains("12.4k"), "{wide}");
    let narrow: String = block_cells(&p, 18, 0, 0)
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.c)
        .collect();
    assert!(!narrow.contains("12.4k"), "{narrow}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatswarmview`
Expected: FAIL to compile (module doesn't exist).

- [ ] **Step 3: Implement `chatswarmview.rs` and integrate**

Create `crates/crew-app/src/chatswarmview.rs`:

```rust
//! Draws the live swarm-run block at the bottom of the chat message area:
//! one row per task — state glyph (running tasks animate a spinner), title,
//! right-aligned token count. State lives in `chatswarm`; when the run ends
//! the block folds into the transcript, so this only ever draws live runs.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatswarm::glyph;
use crew_hive::TaskState;

/// Most task rows the block will occupy; larger plans get a `… n more` row.
const MAX_ROWS: u16 = 8;
/// Below this width the token column is dropped (title needs the room).
const TOKENS_MIN_COLS: u16 = 24;

/// Rows the live block occupies in the message area (0 = no live run).
pub(crate) fn swarm_rows(pane: &ChatPane, _rows: u16) -> u16 {
    match &pane.swarm {
        Some(s) => (s.tasks.len() as u16).min(MAX_ROWS),
        None => 0,
    }
}

fn fmt_tok(n: u64) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

fn push_str(v: &mut Vec<CellView>, col: &mut u16, row: u16, s: &str, fg: (u8, u8, u8)) {
    for c in s.chars() {
        v.push(CellView {
            col: *col,
            row,
            c,
            fg,
            bg: crew_theme::theme().page_bg,
            bold: false,
            italic: false,
        });
        *col += 1;
    }
}

/// Render the block, one task per row starting at `top_row`. `now_ms` drives
/// the running-task spinner (0 in tests = first frame).
pub(crate) fn block_cells(pane: &ChatPane, cols: u16, top_row: u16, now_ms: u64) -> Vec<CellView> {
    let Some(s) = &pane.swarm else {
        return Vec::new();
    };
    let theme = crew_theme::theme();
    let mut v = Vec::new();
    let shown = (s.tasks.len()).min(MAX_ROWS as usize);
    // With more tasks than rows, the last row becomes the overflow summary.
    let listed = if s.tasks.len() > shown { shown - 1 } else { shown };
    for (i, t) in s.tasks.iter().take(listed).enumerate() {
        let row = top_row + i as u16;
        let (g, fg) = match t.state {
            TaskState::Running => {
                let f = (now_ms / 120) as usize % crate::update::SPINNER.len();
                (crate::update::SPINNER[f], crate::palette::accent())
            }
            TaskState::Done => (glyph(&t.state), theme.activity),
            TaskState::Failed => (glyph(&t.state), theme.bell),
            _ => (glyph(&t.state), theme.text_muted),
        };
        let mut col = 1u16;
        push_str(&mut v, &mut col, row, &g.to_string(), fg);
        push_str(&mut v, &mut col, row, " ", fg);
        // Title, clamped to leave room for the token column (or the edge).
        let tok = (t.tokens > 0 && cols >= TOKENS_MIN_COLS).then(|| fmt_tok(t.tokens));
        let reserve = tok.as_ref().map(|s| s.len() as u16 + 2).unwrap_or(1);
        let max_title = cols.saturating_sub(col + reserve) as usize;
        let title: String = t.title.chars().take(max_title).collect();
        push_str(&mut v, &mut col, row, &title, theme.text_muted);
        if let Some(tok) = tok {
            let mut tcol = cols.saturating_sub(tok.len() as u16 + 1);
            push_str(&mut v, &mut tcol, row, &tok, theme.text_muted);
        }
    }
    if s.tasks.len() > shown {
        let more = s.tasks.len() - listed;
        let mut col = 1u16;
        push_str(
            &mut v,
            &mut col,
            top_row + listed as u16,
            &format!("… {more} more"),
            theme.text_muted,
        );
    }
    v
}

#[cfg(test)]
#[path = "chatswarmview_tests.rs"]
mod tests;
```

Check `crate::update::SPINNER`'s element type (`chatchips` indexes it as `SPINNER[f]` inside a `format!` — if elements are `&str`, use `push_str(.., SPINNER[f], ..)` directly instead of `.to_string()`).

Integrate:

1. `chatplace.rs` `msg_rows_budget` — the block claims rows from the message body so nothing overdraws:

```rust
pub(crate) fn msg_rows_budget(pane: &ChatPane, cols: u16, rows: u16) -> u16 {
    let top = pane.status_rows(cols, rows);
    let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
    let block = crate::chatswarmview::swarm_rows(pane, rows);
    rows.saturating_sub(top + bottom + block)
}
```

2. `chatview.rs` `cells` — draw the block between the message area and the composer. In the `else` branch (messages non-empty), after the `new_pill_cells` block, add:

```rust
        // The live swarm block sits under the messages, above the composer —
        // msg_rows_budget already reserved its rows so nothing overlaps.
        cells.extend(crate::chatswarmview::block_cells(
            pane,
            cols,
            top + msg_rows,
            crate::anim::now_ms(),
        ));
```

   Also handle the `pane.messages.is_empty()` branch (a run can start before any reply lands — the plan-summary message usually exists, but don't rely on it): add the same `block_cells` call after `empty_cells`, at row `rows - bottom - crate::chatswarmview::swarm_rows(pane, rows)`.

3. `main.rs`: add `mod chatswarmview;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: ALL PASS (chatswarmview tests plus no regressions in chatview/chatplace tests).

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-app/src/chatswarmview.rs crates/crew-app/src/chatswarmview_tests.rs crates/crew-app/src/chatplace.rs crates/crew-app/src/chatview.rs crates/crew-app/src/main.rs
git commit -m "feat(chat): live swarm status block renders in the chat pane

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: `[-][x]` border buttons in panecard

**Files:**
- Modify: `crates/crew-app/src/panecard.rs` (button layout + `close_btn_rect`)
- Modify: `crates/crew-app/src/paneview_tests.rs` (update positions; add close-rect tests)

**Interfaces:**
- Consumes: nothing new.
- Produces (Task 7 relies on): `pub(crate) fn close_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect>` (the `[x]`, corner slot, card columns `cols-5 ..= cols-3`); `min_btn_rect` MOVES to card columns `cols-8 ..= cols-6`; both gated by `const BTNS_COLS: u16 = 13`.

- [ ] **Step 1: Update/write the failing tests**

In `crates/crew-app/src/paneview_tests.rs`, find the existing `min_btn`/`min_rect` tests (`min_btn_draws_on_the_top_border_and_shifts_status_glyphs`, `min_btn_absent_when_disabled_or_narrow`, `min_btn_rect_covers_the_button_cells` — names from the earlier `-r` mangling may differ slightly; locate with `rg -n "min_btn" crates/crew-app/src/paneview_tests.rs`). Update them for the new layout and add close-button coverage:

```rust
#[test]
fn border_buttons_draw_minus_then_x_and_shift_status_glyphs() {
    let b = bar_with_min_btn(); // reuse the existing test helper/Bar literal, min_btn: true
    let v = pane_card(20, 5, &b); // 22 card cols ≥ BTNS_COLS
    let row0: String = row_chars(&v, 0); // existing helper, or collect col-sorted row-0 chars
    assert!(row0.contains("[-][x]"), "{row0}");
}

#[test]
fn border_buttons_absent_when_narrow() {
    let b = bar_with_min_btn();
    let v = pane_card(9, 5, &b); // 11 card cols < BTNS_COLS = 13
    let row0: String = row_chars(&v, 0);
    assert!(!row0.contains('x') && !row0.contains('-'), "{row0}");
}

#[test]
fn close_rect_covers_the_corner_button_and_min_rect_sits_left_of_it() {
    let r = Rect { x: 0.0, y: 0.0, w: 300.0, h: 100.0 };
    let close = close_btn_rect(r, 10.0, 20.0).unwrap();
    let min = min_btn_rect(r, 10.0, 20.0).unwrap();
    assert_eq!(close.w, 30.0);
    assert_eq!(min.w, 30.0);
    // [x] takes the corner slot; [-] sits directly left of it.
    assert!((min.x + 30.0 - close.x).abs() < f32::EPSILON);
    assert!(close.x > min.x);
}
```

Match the existing tests' helper names for constructing `Bar` and extracting row-0 text — reuse them rather than inventing new ones.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app paneview`
Expected: FAIL — `close_btn_rect` undefined; `[-][x]` not drawn.

- [ ] **Step 3: Implement the button pair**

In `crates/crew-app/src/panecard.rs`:

1. Replace `MIN_BTN_COLS` with:

```rust
/// Narrowest card (in cells, border included) that carries the border
/// buttons `[-][x]` — below this there's no room for legible click targets,
/// and the pair draws all-or-nothing so hit-tests never half-apply.
const BTNS_COLS: u16 = 13;
```

2. Update the `Bar.min_btn` doc comment to mention both buttons (draw + `min_btn_rect`/`close_btn_rect` share `BTNS_COLS`).

3. Replace `min_btn_rect` and add `close_btn_rect`:

```rust
/// Pixel rect of one 3-cell border button whose leftmost glyph sits at card
/// column `cols - off`. `None` when the card is too narrow for the pair.
fn btn_rect(rect: Rect, cw: f32, ch: f32, off: u16) -> Option<Rect> {
    let (icols, _) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);
    let cols = icols + 2;
    if cols < BTNS_COLS {
        return None;
    }
    Some(Rect {
        x: rect.x + f32::from(cols - off) * cw,
        y: rect.y,
        w: 3.0 * cw,
        h: ch,
    })
}

/// The `[x]` close button: the corner slot (card columns `cols-5 ..= cols-3`).
pub(crate) fn close_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect> {
    btn_rect(rect, cw, ch, 5)
}

/// The `[-]` minimize button, directly left of `[x]` (columns `cols-8 ..= cols-6`).
pub(crate) fn min_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect> {
    btn_rect(rect, cw, ch, 8)
}
```

4. In `pane_card`, replace the `[-]` drawing block (lines 110-116) with:

```rust
    // The [-][x] buttons claim the corner slots; status glyphs step past them.
    if b.min_btn && cols >= BTNS_COLS {
        for (i, ch) in "[-][x]".chars().enumerate() {
            put(&mut v, cols - 8 + i as u16, 0, ch, legend);
        }
        rx = cols.saturating_sub(10);
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: ALL PASS (paneview tests updated, no other test reads button geometry).

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-app/src/panecard.rs crates/crew-app/src/paneview_tests.rs
git commit -m "feat(app): [x] close button joins [-] on every pane border

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 7: Wire close: click on `[x]`, Esc on swarm panes

**Files:**
- Modify: `crates/crew-app/src/hit.rs` (add `close_btn_at_cursor`, share the tile-scan with `min_btn_at_cursor`)
- Modify: `crates/crew-app/src/events.rs` (click routing — close wins before minimize)
- Modify: `crates/crew-app/src/keys.rs` (Swarm arm: Esc closes)
- Modify: `crates/crew-app/src/swarmpane.rs` (+ its test block or `swarm/tests.rs`): `esc_closes`

**Interfaces:**
- Consumes: `close_btn_rect`/`min_btn_rect` from Task 6; `close_pane` from Task 4.
- Produces: `pub(crate) fn esc_closes(key: &winit::keyboard::Key, pressed: bool) -> bool` in `swarmpane.rs`.

- [ ] **Step 1: Write the failing test for `esc_closes`**

In `crates/crew-app/src/swarm/tests.rs` (where SwarmPane tests live), the winit `KeyEvent` can't be constructed directly — so make `esc_closes` take the parts keys.rs has, keeping it trivially testable:

```rust
// swarmpane.rs — the swarm view has no cursor or input; the only key it
// answers is Escape → close, matching the Far/Markdown/Chat panes.
pub(crate) fn esc_closes(key: &winit::keyboard::Key, pressed: bool) -> bool {
    pressed && matches!(key, winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape))
}
```

Test (in `swarm/tests.rs`):

```rust
#[test]
fn escape_closes_a_swarm_pane_other_keys_do_not() {
    use winit::keyboard::{Key, NamedKey};
    assert!(crate::swarmpane::esc_closes(&Key::Named(NamedKey::Escape), true));
    assert!(!crate::swarmpane::esc_closes(&Key::Named(NamedKey::Escape), false));
    assert!(!crate::swarmpane::esc_closes(&Key::Named(NamedKey::Enter), true));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app escape_closes_a_swarm_pane`
Expected: FAIL — `esc_closes` undefined.

- [ ] **Step 3: Implement the wiring**

1. `swarmpane.rs`: add `esc_closes` as above.

2. `keys.rs`: in the focused-pane match, replace the Swarm arm and handle it after the match (following the `far_action` pattern):

```rust
        let mut swarm_close = false;
        …
                // The swarm view is display-only; Escape closes it.
                PaneContent::Swarm(_) => {
                    swarm_close =
                        crate::swarmpane::esc_closes(&event.logical_key, event.state.is_pressed());
                }
        …
        if swarm_close {
            self.close_pane(focused);
        }
```

3. `hit.rs`: factor the tile scan so both buttons share it, and add the close probe:

```rust
    /// Which full tile's border button (per `rect_of`) is under the cursor.
    /// Zoomed, the one expanded tile carries the buttons; in the grid only
    /// the full tiles do (strip thumbnails draw none, so are never tested).
    fn border_btn_at_cursor(
        &self,
        rect_of: fn(crate::layout::Rect, f32, f32) -> Option<crate::layout::Rect>,
    ) -> Option<usize> {
        let (cw, ch, _sw, _sh, _scale) = self.frame_geometry()?;
        let (content, placed) = self.placed_grid()?;
        let tiles = if self.zoomed {
            crate::render::frame_hit_rects(true, self.focused, self.panes.len(), content, placed)
        } else {
            placed.full
        };
        tiles.into_iter().find_map(|(idx, r)| {
            let hit = rect_of(r, cw, ch)?;
            chrome::point_in(hit, self.cursor.0, self.cursor.1).then_some(idx)
        })
    }

    pub(crate) fn min_btn_at_cursor(&self) -> Option<usize> {
        self.border_btn_at_cursor(crate::panecard::min_btn_rect)
    }

    pub(crate) fn close_btn_at_cursor(&self) -> Option<usize> {
        self.border_btn_at_cursor(crate::panecard::close_btn_rect)
    }
```

(Keep `min_btn_at_cursor`'s existing doc comment on `border_btn_at_cursor`.)

4. `events.rs`: in the left-button `Pressed` arm, before the `min_btn_at_cursor` check (events.rs:52):

```rust
                // The [x] border button closes the pane outright; like [-] it
                // must win over focus/drag so the click does nothing else.
                if let Some(i) = self.close_btn_at_cursor() {
                    self.close_pane(i);
                    self.redraw();
                    return;
                }
```

- [ ] **Step 4: Run the app test suite**

Run: `cargo test -p crew-app`
Expected: ALL PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-app/src/hit.rs crates/crew-app/src/events.rs crates/crew-app/src/keys.rs crates/crew-app/src/swarmpane.rs crates/crew-app/src/swarm/tests.rs
git commit -m "feat(app): click [x] closes any pane; Esc closes swarm panes

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 8: Full-suite check + live GUI verification

**Files:** none (verification only; fix regressions where found).

- [ ] **Step 1: Full workspace test run**

Run: `cargo test --workspace`
Expected: ALL PASS.

- [ ] **Step 2: Live GUI verification (verify skill)**

Follow `.claude/skills/verify` (isolated-HOME dev instance, frontmost-PID guard). With the mock broker (`CREW_BROKER_MOCK_REPLY`), open a `/crew` chat pane and send a plain task. Confirm by screenshot:

1. NO second "hive" pane appears.
2. The status block appears at the bottom of the chat message area (task rows with glyphs), then disappears, leaving a `┆ crew` record message with `✓` lines in the transcript, followed by the swarm-done summary.
3. Every pane's border shows `[-][x]`; clicking `[x]` closes that pane; clicking `[-]` still minimizes.
4. Open a `/swarm` batch pane; press Esc → it closes.

Note: the live app spawns the INSTALLED broker (`~/.local/bin/crew`) — build and install the dev broker first (`cargo build --bin crew -p crew-plugin` and copy per the verify skill), or the live-drain change won't be exercised.

- [ ] **Step 3: Commit any verification fixes; hand off**

Use superpowers:finishing-a-development-branch — local no-ff merge into main, delete the branch, offer (never auto) push.
