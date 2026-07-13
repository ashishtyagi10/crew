# crew-hive as Default /crew Engine — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Plain messages typed into the `/crew` chat pane are dynamically planned and executed as crew-hive agent swarms inside the broker, streamed back as chat events plus a live companion swarm-graph pane; `@agent` addressing still dials directly.

**Architecture:** The broker's per-task worker thread runs plan→schedule→execute on a current-thread tokio runtime (mirroring `crew-app/src/swarm/bridge.rs`), translating `HiveEvent`s into existing `PluginEvent`s and forwarding them raw via new `PluginEvent::Hive`/`HivePlan` variants. The app routes those to a reusable swarm pane labeled `"hive"`. Two crew-hive generalizations (blanket `Arc` Provider impl, model-id override) make the engine work on the broker's DashScope/OpenRouter/Anthropic providers.

**Tech Stack:** Rust workspace; crates: `crew-hive`, `crew-plugin`, `crew-app`; tokio current-thread runtimes; serde JSON line protocol.

**Spec:** `docs/superpowers/specs/2026-07-12-crew-hive-default-engine-design.md`

## Global Constraints

- Budget/concurrency: $1.00 per task run (`1_000_000` micros-USD), concurrency `4`, worker `max_tokens` `2048`, plan tier `ModelTier::Standard`.
- LLM-derived plans stay `AgentKind::Api` only (existing `parse_plan` invariant — do not touch).
- Never block the winit thread: all app-side handling stays in the existing non-blocking `poll()` paths.
- The broker is a subprocess; its worker threads may block.
- Every commit must pass the repo pre-commit hook (`cargo fmt` check + `cargo check`). Run `cargo fmt` before each commit.
- Do NOT modify the relay engine (`engine.rs`), `fan.rs`, or `commands.rs` behavior — `@agent`, `@a+b`, and `/command` paths must work unchanged.
- Tests must not depend on env vars: inject planner/factory (see Task 4's `run_with`).

---

### Task 1: crew-hive — `Provider` blanket impl for `Arc<P>`

**Files:**
- Modify: `crates/crew-hive/src/provider/mod.rs` (append after the `Provider` trait)
- Test: `crates/crew-hive/src/provider/tests.rs`

**Interfaces:**
- Produces: `impl<P: Provider + ?Sized> Provider for Arc<P>` — lets `LlmPlanner<Arc<dyn Provider>>` (Task 4) type-check.

- [ ] **Step 1: Write the failing test** (append to `crates/crew-hive/src/provider/tests.rs`)

```rust
#[tokio::test]
async fn arc_dyn_provider_is_a_provider() {
    // The broker holds Arc<dyn Provider>; LlmPlanner<P: Provider> must accept it.
    fn takes_provider<P: crate::provider::Provider>(p: P) -> P {
        p
    }
    let arc: std::sync::Arc<dyn crate::provider::Provider> =
        std::sync::Arc::new(crate::provider::MockProvider {
            reply: "ok".into(),
        });
    let p = takes_provider(arc);
    let got = p
        .complete(crate::provider::CompletionRequest {
            model: "mock".into(),
            system: None,
            prompt: "hi".into(),
            max_tokens: 16,
        })
        .await
        .unwrap();
    assert_eq!(got.text, "ok");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-hive arc_dyn_provider_is_a_provider 2>&1 | tail -5`
Expected: COMPILE FAIL — `the trait bound Arc<dyn Provider>: Provider is not satisfied`.

- [ ] **Step 3: Write minimal implementation** (append to `crates/crew-hive/src/provider/mod.rs`)

```rust
/// `Arc<dyn Provider>` (and any `Arc<P>`) is itself a Provider, so callers
/// that hold a dynamically-discovered provider (the broker) can feed it to
/// generic consumers like `LlmPlanner<P: Provider>` without re-wrapping.
impl<P: Provider + ?Sized> Provider for std::sync::Arc<P> {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        (**self).complete(req)
    }

    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        (**self).complete_streaming(req, on_chunk)
    }
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p crew-hive provider 2>&1 | tail -5`
Expected: PASS (all provider tests).

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-hive/src/provider/ && git commit -m "feat(hive): Arc<P> blanket Provider impl for dynamically discovered providers"
```

---

### Task 2: crew-hive — model-id override on `LlmPlanner` and `ApiFactory`/`ApiAgent`

**Files:**
- Modify: `crates/crew-hive/src/planner/mod.rs` (LlmPlanner struct + plan())
- Modify: `crates/crew-hive/src/apiagent/mod.rs` (ApiAgent + ApiFactory)
- Modify: `crates/crew-app/src/swarmpane.rs:78-81` (struct literal gains `model: None`)
- Test: `crates/crew-hive/src/planner/tests.rs` (or the `#[cfg(test)] mod tests` in `planner/mod.rs` if that's where existing planner tests live — follow the existing location), `crates/crew-hive/src/apiagent/tests.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `LlmPlanner { provider: P, tier: ModelTier, model: Option<String> }` with `pub fn with_model(self, m: impl Into<String>) -> Self`. `plan()` uses `self.model.clone().unwrap_or_else(|| self.tier.model_id().to_owned())`.
  - `ApiFactory::with_model(self, m: impl Into<String>) -> Self`; made agents send that model id instead of `tier.model_id()` (tier still drives `cost_micros`).
  - `ApiAgent::new(provider, max_tokens)` unchanged; gains `pub fn with_model(self, m: impl Into<String>) -> Self`.

- [ ] **Step 1: Write the failing tests**

Planner test (next to existing planner tests):

```rust
#[tokio::test]
async fn llm_planner_model_override_reaches_request() {
    use std::sync::{Arc, Mutex};
    // A probe provider that records the requested model and returns a valid plan.
    struct Probe(Arc<Mutex<String>>);
    impl crate::provider::Provider for Probe {
        fn complete(
            &self,
            req: crate::provider::CompletionRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::provider::Completion,
                            crate::provider::ProviderError,
                        >,
                    > + Send,
            >,
        > {
            *self.0.lock().unwrap() = req.model.clone();
            Box::pin(async {
                Ok(crate::provider::Completion {
                    text: r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#.into(),
                    input_tokens: 1,
                    output_tokens: 1,
                })
            })
        }
    }
    let seen = Arc::new(Mutex::new(String::new()));
    let planner = LlmPlanner {
        provider: Probe(seen.clone()),
        tier: crate::graph::ModelTier::Standard,
        model: None,
    }
    .with_model("qwen-max");
    planner.plan("goal").await.unwrap();
    assert_eq!(seen.lock().unwrap().as_str(), "qwen-max");
}
```

ApiAgent test (in `apiagent/tests.rs`, mirroring its existing test harness — it already runs agents against `MockProvider`; add a Probe like above wrapped as `Arc<dyn Provider>`):

```rust
#[tokio::test]
async fn api_factory_model_override_reaches_request() {
    use std::sync::{Arc, Mutex};
    struct Probe(Arc<Mutex<String>>);
    impl crate::provider::Provider for Probe {
        fn complete(
            &self,
            req: crate::provider::CompletionRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::provider::Completion,
                            crate::provider::ProviderError,
                        >,
                    > + Send,
            >,
        > {
            *self.0.lock().unwrap() = req.model.clone();
            Box::pin(async {
                Ok(crate::provider::Completion {
                    text: "done".into(),
                    input_tokens: 1,
                    output_tokens: 1,
                })
            })
        }
    }
    let seen = Arc::new(Mutex::new(String::new()));
    let provider: Arc<dyn crate::provider::Provider> = Arc::new(Probe(seen.clone()));
    let factory = ApiFactory::new(provider, 64).with_model("qwen-max");
    let agent = factory.make(&crate::graph::AgentKind::Api { system: None });
    // Build the AgentContext the same way this file's existing MockProvider
    // tests do (a one-task spec, a fresh EventBus, empty deps) — copy that
    // construction verbatim from the test directly above this one — then:
    let _result = agent.run(ctx).await;
    assert_eq!(seen.lock().unwrap().as_str(), "qwen-max");
}
```

The existing tests in `apiagent/tests.rs` already construct `AgentContext` (task spec + bus + deps) for `MockProvider` runs — reuse that exact construction for `ctx`; only the provider and the final assertion differ from the neighboring test.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-hive model_override 2>&1 | tail -8`
Expected: COMPILE FAIL — no field `model` on `LlmPlanner`, no method `with_model`.

- [ ] **Step 3: Implement**

`planner/mod.rs` — struct + builder + plan():

```rust
pub struct LlmPlanner<P: Provider> {
    pub provider: P,
    pub tier: ModelTier,
    /// When set, overrides `tier.model_id()` in the planning request —
    /// required for non-Anthropic providers (DashScope/OpenRouter), whose
    /// model ids the tier table doesn't know.
    pub model: Option<String>,
}

impl<P: Provider> LlmPlanner<P> {
    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }
}
```

In `plan()` replace `model: self.tier.model_id().to_owned(),` with:

```rust
model: self
    .model
    .clone()
    .unwrap_or_else(|| self.tier.model_id().to_owned()),
```

`apiagent/mod.rs` — ApiAgent gains the override:

```rust
pub struct ApiAgent {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
}

impl ApiAgent {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
        }
    }

    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }
}
```

In `ApiAgent::run`, capture `let model = self.model.clone();` before the `Box::pin` and replace `model: tier.model_id().to_owned(),` with:

```rust
model: model.unwrap_or_else(|| tier.model_id().to_owned()),
```

`ApiFactory` mirrors it:

```rust
pub struct ApiFactory {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
}

impl ApiFactory {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
        }
    }

    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }
}

impl AgentFactory for ApiFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        let agent = ApiAgent::new(Arc::clone(&self.provider), self.max_tokens);
        Box::new(match &self.model {
            Some(m) => agent.with_model(m.clone()),
            None => agent,
        })
    }
}
```

`crates/crew-app/src/swarmpane.rs` (the `/goal` literal, ~line 78):

```rust
let planner = Arc::new(LlmPlanner {
    provider: provider.clone(),
    tier: PLAN_TIER,
    model: None,
});
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-hive 2>&1 | tail -3 && cargo test -p crew-app --bin crew swarm 2>&1 | tail -3`
Expected: PASS on both.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-hive/src/planner/ crates/crew-hive/src/apiagent/ crates/crew-app/src/swarmpane.rs && git commit -m "feat(hive): model-id override on LlmPlanner and ApiFactory for non-Anthropic providers"
```

---

### Task 3: crew-plugin — `PluginEvent::HivePlan` + `PluginEvent::Hive`

**Files:**
- Modify: `crates/crew-plugin/src/protocol.rs` (add two variants + tests in the file's `mod tests`)

**Interfaces:**
- Consumes: `crew_hive::{HiveEvent, TaskSpec}` (both already `Serialize + Deserialize`). `crew-plugin` already depends on `crew-hive`.
- Produces (for Tasks 4, 5, 6):

```rust
/// A swarm plan landed: the full task list, so the host can open/refresh
/// the companion graph pane. Sent once per swarm run, before execution.
HivePlan {
    tasks: Vec<crew_hive::TaskSpec>,
},
/// One raw swarm telemetry event, forwarded verbatim for the host's
/// companion graph pane. Chat-facing translations are sent separately.
Hive {
    event: crew_hive::HiveEvent,
},
```

- [ ] **Step 1: Write the failing round-trip test** (in `protocol.rs`'s `mod tests`)

```rust
#[test]
fn hive_events_round_trip() {
    let plan = PluginEvent::HivePlan {
        tasks: vec![crew_hive::TaskSpec {
            id: crew_hive::TaskId(0),
            title: "t".into(),
            agent: crew_hive::AgentKind::Api { system: None },
            model: crew_hive::ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
        }],
    };
    let s = serde_json::to_string(&plan).unwrap();
    assert!(s.contains("\"type\":\"hive_plan\""), "{s}");
    let ev = PluginEvent::Hive {
        event: crew_hive::HiveEvent::TaskStateChanged {
            task: crew_hive::TaskId(0),
            state: crew_hive::TaskState::Running,
        },
    };
    let s = serde_json::to_string(&ev).unwrap();
    let back: PluginEvent = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, PluginEvent::Hive { .. }));
}
```

(If `TaskId`/`TaskState`/`TaskSpec`/`AgentKind` aren't re-exported from `crew_hive`'s root, check `crates/crew-hive/src/lib.rs:38-96` — they are listed there; use whatever path the re-export gives.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-plugin hive_events_round_trip 2>&1 | tail -5`
Expected: COMPILE FAIL — no variant `HivePlan`.

- [ ] **Step 3: Add the two variants** to `PluginEvent` (before `Error`), exactly as in **Interfaces** above.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-plugin 2>&1 | tail -3`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/protocol.rs && git commit -m "feat(plugin): HivePlan/Hive protocol events for swarm telemetry"
```

---

### Task 4: broker swarm driver (`swarm.rs`) + stdio routing

**Files:**
- Create: `crates/crew-plugin/src/broker/swarm.rs`
- Modify: `crates/crew-plugin/src/broker/mod.rs` (add `mod swarm;` to the module list)
- Modify: `crates/crew-plugin/src/broker/stdio.rs` (worker dispatch ~line 229-233; make `msg` and `relay_counting` visible to the new module if needed — `msg` is already used across the file; mark it `pub(crate)` if it isn't)
- Test: `#[cfg(test)] mod tests` at the bottom of `swarm.rs`

**Interfaces:**
- Consumes: `discover::provider_and_model()` (`discover.rs:155`), `Session` (`session.rs`: `pub cancel: Arc<AtomicBool>`, `session.cancelled()`), `PluginEvent`, Task 1's `Arc` impl, Task 2's `with_model`, Task 3's variants, `crew_hive::{Planner, StubPlanner, LlmPlanner, TaskGraph, TaskSpec, TaskId, TaskState, AgentId, HiveEvent, EventBus, Blackboard, Scheduler, budget_governor, Budget, AgentFactory, ApiFactory, StubFactory (at crew_hive::agent::StubFactory), ModelTier}`.
- Produces (consumed by stdio dispatch):

```rust
pub(crate) fn run_task(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()>
```

- [ ] **Step 1: Write the failing tests** (bottom of the new `swarm.rs`; they target the injectable core `run_with`, no env vars)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crew_hive::agent::StubFactory;
    use crew_hive::StubPlanner;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn collect(task: &str, cancel: Arc<AtomicBool>) -> Vec<PluginEvent> {
        let mut evs = Vec::new();
        run_with(
            task,
            Arc::new(StubPlanner { fanout: 2 }),
            Arc::new(StubFactory),
            None,
            cancel,
            &mut |ev| {
                evs.push(ev);
                Ok(())
            },
        )
        .unwrap();
        evs
    }

    #[test]
    fn plain_task_emits_plan_then_hive_events_then_summary() {
        let evs = collect("build the thing", Arc::new(AtomicBool::new(false)));
        // A HivePlan with 3 tasks (2 leaves + merge) is announced first.
        assert!(matches!(
            evs.first(),
            Some(PluginEvent::HivePlan { tasks }) if tasks.len() == 3
        ));
        // Raw telemetry flows for the graph pane.
        assert!(evs.iter().any(|e| matches!(e, PluginEvent::Hive { .. })));
        // A plan-summary chat message names the tasks.
        assert!(evs.iter().any(
            |e| matches!(e, PluginEvent::Message { text, .. } if text.contains("leaf-0"))
        ));
        // The final aggregate message closes the run.
        assert!(evs.iter().rev().any(
            |e| matches!(e, PluginEvent::Message { text, .. } if text.contains("swarm done"))
        ));
    }

    #[test]
    fn pre_cancelled_run_reports_cancellation() {
        let evs = collect("task", Arc::new(AtomicBool::new(true)));
        assert!(evs.iter().any(
            |e| matches!(e, PluginEvent::Message { text, .. } if text.contains("cancelled"))
        ));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-plugin swarm 2>&1 | tail -5`
Expected: COMPILE FAIL — module/function missing.

- [ ] **Step 3: Implement `swarm.rs`**

```rust
//! Default /crew execution: plan a plain message into a crew-hive task
//! graph and run it as a swarm on this worker thread, streaming chat
//! events plus raw Hive telemetry for the host's companion graph pane.
//! `@agent` addressing bypasses this module (stdio routes it to the relay).
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crew_hive::agent::StubFactory;
use crew_hive::{
    budget_governor, AgentFactory, AgentId, Blackboard, Budget, EventBus, HiveEvent, LlmPlanner,
    ModelTier, Planner, Scheduler, StubPlanner, TaskGraph, TaskId, TaskState,
};

use crate::protocol::PluginEvent;

use super::session::Session;
use super::stdio::msg;

/// $1.00 ceiling per swarm run, enforced by the budget governor.
const SWARM_BUDGET_MICROS_USD: u64 = 1_000_000;
/// Parallel worker agents per run.
const CONCURRENCY: usize = 4;
/// Per-task output token cap for worker agents.
const WORK_MAX_TOKENS: u32 = 2048;
/// Fan-out for the offline stub planner.
const STUB_FANOUT: usize = 2;

/// Pick planner/factory/budget from provider discovery: real LLM planning on
/// a discovered provider; deterministic stubs when keyless. The mock provider
/// (GUI harness) plans with stubs but executes through the mock, so replies
/// stay deterministic while the full pipeline runs.
fn backend() -> (Arc<dyn Planner>, Arc<dyn AgentFactory>, Option<Budget>) {
    match super::discover::provider_and_model() {
        None => (
            Arc::new(StubPlanner {
                fanout: STUB_FANOUT,
            }),
            Arc::new(StubFactory),
            None,
        ),
        Some((provider, model)) if model == "mock" => (
            Arc::new(StubPlanner {
                fanout: STUB_FANOUT,
            }),
            Arc::new(crew_hive::ApiFactory::new(provider, WORK_MAX_TOKENS)),
            None,
        ),
        Some((provider, model)) => (
            Arc::new(
                LlmPlanner {
                    provider: Arc::clone(&provider),
                    tier: ModelTier::Standard,
                    model: None,
                }
                .with_model(model.clone()),
            ),
            Arc::new(crew_hive::ApiFactory::new(provider, WORK_MAX_TOKENS).with_model(model)),
            Some(Budget {
                max_micros_usd: SWARM_BUDGET_MICROS_USD,
            }),
        ),
    }
}

/// Entry point for a plain (unaddressed) chat task.
pub(crate) fn run_task(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    super::sessionlog::append("user", task);
    let (planner, factory, budget) = backend();
    run_with(
        task,
        planner,
        factory,
        budget,
        Arc::clone(&session.cancel),
        emit,
    )
}

/// Injectable core: plan `task`, execute the graph, translate events.
pub(crate) fn run_with(
    task: &str,
    planner: Arc<dyn Planner>,
    factory: Arc<dyn AgentFactory>,
    budget: Option<Budget>,
    cancel: Arc<AtomicBool>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    emit(PluginEvent::Activity {
        agent: "planner".into(),
        state: "thinking".into(),
        from: "user".into(),
    })?;
    let graph = match rt.block_on(planner.plan(task)) {
        Ok(g) => g,
        Err(e) => {
            emit(msg("crew", format!("planning failed ({e}) — answering directly")))?;
            emit(PluginEvent::Activity {
                agent: String::new(),
                state: "idle".into(),
                from: String::new(),
            })?;
            // Degrade to a single-task graph so chat never dead-ends.
            let single = crew_hive::TaskSpec {
                id: TaskId(0),
                title: "reply".into(),
                agent: crew_hive::AgentKind::Api { system: None },
                model: ModelTier::Standard,
                deps: vec![],
                prompt: task.to_owned(),
            };
            TaskGraph::new(vec![single]).expect("single task graph is valid")
        }
    };

    let tasks: Vec<crew_hive::TaskSpec> = graph.tasks().to_vec();
    let titles: HashMap<TaskId, String> =
        tasks.iter().map(|t| (t.id, t.title.clone())).collect();
    emit(PluginEvent::HivePlan {
        tasks: tasks.clone(),
    })?;
    emit(msg(
        "crew",
        format!(
            "planned {} task(s): {}",
            tasks.len(),
            tasks
                .iter()
                .map(|t| t.title.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))?;

    // Execute: scheduler + optional budget governor + bus drain, all on this
    // thread's runtime (the pattern proven in crew-app/src/swarm/bridge.rs).
    let board = Blackboard::new();
    let bus = EventBus::new(256);
    let mut sub = bus.subscribe();
    let governor = budget.map(|b| budget_governor(bus.clone(), b, Arc::clone(&cancel)));
    let sched = Scheduler::new(graph.clone(), board.clone(), bus, factory, CONCURRENCY)
        .with_cancel(Arc::clone(&cancel));

    let (tx, rx) = std::sync::mpsc::channel::<HiveEvent>();
    let outcome = rt.block_on(async move {
        let drain = async move {
            while let Ok(ev) = sub.recv().await {
                if tx.send(ev).is_err() {
                    break;
                }
            }
        };
        match governor {
            Some(g) => tokio::join!(sched.run(), drain, g).0,
            None => tokio::join!(sched.run(), drain).0,
        }
    });

    // Translate the drained telemetry (the run has completed; the channel
    // holds the full ordered stream).
    let mut agent_task: HashMap<u64, TaskId> = HashMap::new();
    while let Ok(ev) = rx.try_recv() {
        emit(PluginEvent::Hive { event: ev.clone() })?;
        for out in translate(&ev, &titles, &mut agent_task) {
            emit(out)?;
        }
    }

    // Final aggregate: sink tasks' outputs (tasks nothing depends on).
    let sink_ids: Vec<TaskId> = tasks
        .iter()
        .filter(|t| !tasks.iter().any(|o| o.deps.contains(&t.id)))
        .map(|t| t.id)
        .collect();
    let sinks = rt.block_on(board.gather(&sink_ids));
    let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);
    let mut summary = if cancelled {
        format!(
            "swarm cancelled (budget or /stop) — {} done, {} failed, {} cancelled",
            outcome.done.len(),
            outcome.failed.len(),
            outcome.cancelled.len()
        )
    } else {
        format!(
            "swarm done — {} task(s), {} failed",
            outcome.done.len(),
            outcome.failed.len()
        )
    };
    for r in &sinks {
        if r.success && !r.output.is_empty() {
            summary.push_str("\n\n");
            summary.push_str(&r.output);
        }
    }
    emit(msg("crew", summary))?;
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    Ok(())
}

/// Map one HiveEvent to chat-facing events. Raw `Hive` forwarding happens at
/// the call site; this returns only the human-readable translations.
fn translate(
    ev: &HiveEvent,
    titles: &HashMap<TaskId, String>,
    agent_task: &mut HashMap<u64, TaskId>,
) -> Vec<PluginEvent> {
    let title_of = |t: &TaskId| {
        titles
            .get(t)
            .cloned()
            .unwrap_or_else(|| format!("task-{}", t.0))
    };
    let agent_name = |a: &AgentId, agent_task: &HashMap<u64, TaskId>| {
        agent_task
            .get(&a.0)
            .map(title_of)
            .unwrap_or_else(|| format!("agent-{}", a.0))
    };
    match ev {
        HiveEvent::AgentSpawned { agent, task } => {
            agent_task.insert(agent.0, *task);
            vec![PluginEvent::Activity {
                agent: title_of(task),
                state: "thinking".into(),
                from: "hive".into(),
            }]
        }
        HiveEvent::TaskStateChanged { task, state } => match state {
            TaskState::Done | TaskState::Failed | TaskState::Cancelled => {
                vec![PluginEvent::Activity {
                    agent: title_of(task),
                    state: "idle".into(),
                    from: String::new(),
                }]
            }
            _ => vec![],
        },
        HiveEvent::TokenDelta { agent, output, .. } => vec![PluginEvent::StatsTick {
            agent: agent_name(agent, agent_task),
            tokens: u64::from(*output),
        }],
        HiveEvent::CostDelta { .. } => vec![],
        HiveEvent::OutputChunk { agent, text } => vec![msg(
            agent_name(agent, agent_task).as_str(),
            text.clone(),
        )],
        HiveEvent::Failed { agent, error } => vec![PluginEvent::Error {
            message: format!("{}: {error}", agent_name(agent, agent_task)),
        }],
    }
}
```

Notes for the implementer:
- `msg(sender, text)` lives in `stdio.rs`; if it is private, change it to `pub(crate) fn msg(...)` (it builds a `PluginEvent::Message` with the current timestamp). Check its exact signature — if it takes `&str`/`String` adjust call sites here accordingly.
- `graph.tasks()` — confirm the accessor name in `crates/crew-hive/src/graph/mod.rs:11-63` (`tasks()` per the exploration; adjust if it returns `&[TaskSpec]`).
- Add `mod swarm;` to `crates/crew-plugin/src/broker/mod.rs`.
- The drained-after-completion translation means chat `Activity`/`StatsTick` events arrive at run end in mock/stub runs (instantaneous anyway). For live streaming, drain concurrently: replace the post-run `while` with a `std::thread::spawn` that owns a cloned emitter — the existing `tick_emit` pattern in `stdio.rs:198-240` shows how a second writer streams mid-run. Implement the threaded variant directly if straightforward: spawn the drain thread BEFORE `rt.block_on`, translating and emitting through an `Arc<dyn Fn(PluginEvent) + Send + Sync>` (same type as `tick_emit`), and keep only the final summary on the main emitter. The tests above pass either way since they only assert presence/order of plan-first and summary-last.

- [ ] **Step 4: Wire the dispatch** in `stdio.rs` (~line 229):

```rust
let res = if is_cmd {
    super::commands::handle(&mut snap, &trimmed, &tick_emit, &mut counting)
} else if trimmed.starts_with('@') {
    relay_counting(&trimmed, &snap, &tick_emit, &mut counting)
} else {
    super::swarm::run_task(&trimmed, &snap, &mut counting)
};
```

- [ ] **Step 5: Run to verify pass**

Run: `cargo test -p crew-plugin 2>&1 | tail -3`
Expected: PASS (new swarm tests + all existing relay/commands tests untouched).

- [ ] **Step 6: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/ && git commit -m "feat(broker): crew-hive swarms are the default execution for plain /crew tasks"
```

---

### Task 5: app — classify `HivePlan`/`Hive` into `HostAction`s

**Files:**
- Modify: `crates/crew-app/src/chatevents.rs`
- Test: same file's tests (or `crates/crew-app/src/chatevents` tests where `classify` is already tested — follow existing location; `chat_tests.rs` has classify coverage)

**Interfaces:**
- Produces (consumed by Task 7's poll handling):

```rust
pub enum HostAction {
    SpawnPane { command: String, args: Vec<String>, label: String },
    SendPane { label: String, text: String },
    HivePlan { tasks: Vec<crew_hive::TaskSpec> },
    Hive { event: crew_hive::HiveEvent },
}
```

(`crew-app` already depends on `crew-hive`.)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn hive_events_classify_to_host_actions() {
    let ev = PluginEvent::HivePlan { tasks: vec![] };
    assert!(matches!(classify(&ev), Some(HostAction::HivePlan { .. })));
    let ev = PluginEvent::Hive {
        event: crew_hive::HiveEvent::TaskStateChanged {
            task: crew_hive::TaskId(0),
            state: crew_hive::TaskState::Running,
        },
    };
    assert!(matches!(classify(&ev), Some(HostAction::Hive { .. })));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app --bin crew hive_events_classify 2>&1 | tail -5`
Expected: COMPILE FAIL — no variants.

- [ ] **Step 3: Implement** — add the two `HostAction` variants and two `classify` arms:

```rust
PluginEvent::HivePlan { tasks } => Some(HostAction::HivePlan {
    tasks: tasks.clone(),
}),
PluginEvent::Hive { event } => Some(HostAction::Hive {
    event: event.clone(),
}),
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-app --bin crew 2>&1 | tail -3`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-app/src/chatevents.rs && git commit -m "feat(app): classify Hive protocol events into host actions"
```

---

### Task 6: app — remote-fed `SwarmPane` state

**Files:**
- Modify: `crates/crew-app/src/swarmpane.rs` (new state + constructors; extend `poll`, `is_busy`, and the render/cells match)
- Test: `crates/crew-app/src/swarm/tests.rs` (existing swarm test home)

**Interfaces:**
- Consumes: `crew_hive::{TaskSpec, TaskGraph, HiveEvent, Fleet, GraphError}` (already imported in the file).
- Produces (consumed by Task 7):
  - `SwarmPane::for_remote(tasks: Vec<TaskSpec>) -> Result<SwarmPane, GraphError>`
  - `SwarmPane::apply_remote(&mut self, ev: &HiveEvent) -> bool` (true = re-render needed; false when not a Remote pane)

- [ ] **Step 1: Write the failing test** (in `crates/crew-app/src/swarm/tests.rs`)

```rust
#[test]
fn remote_swarm_pane_applies_forwarded_events() {
    use crew_hive::{AgentId, AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
    let tasks = vec![TaskSpec {
        id: TaskId(0),
        title: "t0".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
    }];
    let mut pane = crate::swarmpane::SwarmPane::for_remote(tasks).unwrap();
    assert!(pane.apply_remote(&HiveEvent::AgentSpawned {
        agent: AgentId(1),
        task: TaskId(0),
    }));
    assert!(pane.apply_remote(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    }));
    assert!(pane.is_busy(), "a running remote task means busy");
    assert!(pane.apply_remote(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    }));
    assert!(!pane.is_busy(), "all done \u{2192} idle");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app --bin crew remote_swarm_pane 2>&1 | tail -5`
Expected: COMPILE FAIL — `for_remote` missing.

- [ ] **Step 3: Implement** in `swarmpane.rs`:

Add to `SwarmState`:

```rust
/// Visualising a swarm running elsewhere (the /crew broker); events arrive
/// over the plugin protocol instead of an in-process bus.
Remote { graph: TaskGraph, fleet: Fleet },
```

Constructors/appliers on `SwarmPane`:

```rust
/// Companion view for a broker-side swarm: build the graph from the
/// forwarded plan and wait for `apply_remote` events.
pub fn for_remote(tasks: Vec<crew_hive::TaskSpec>) -> Result<Self, GraphError> {
    Ok(Self {
        state: SwarmState::Remote {
            graph: TaskGraph::new(tasks)?,
            fleet: Fleet::new(),
        },
    })
}

/// Apply one forwarded event. Returns true when this pane is remote-fed
/// (and thus changed); false otherwise so callers can skip a redraw.
pub fn apply_remote(&mut self, ev: &crew_hive::HiveEvent) -> bool {
    if let SwarmState::Remote { fleet, .. } = &mut self.state {
        fleet.apply(ev);
        true
    } else {
        false
    }
}
```

Extend the existing matches:
- `poll()`: `SwarmState::Remote { .. } => false,` (events are pushed by the app, not polled).
- `is_busy()`: `SwarmState::Remote { fleet, .. } => fleet.totals().live > 0,`
- The render path (the match that calls `swarm_cells(handle.graph(), fleet, cols, rows)` for `Running` — find it in this file or `swarm/view.rs` callers): add `SwarmState::Remote { graph, fleet } => swarm_cells(graph, fleet, cols, rows),`

(`Fleet::apply` takes `&HiveEvent` per `telemetry/mod.rs:37-133`; if `fleet.totals().live` isn't the live-count field name, mirror whatever `is_busy` already uses for `Running`.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-app --bin crew 2>&1 | tail -3`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-app/src/swarmpane.rs crates/crew-app/src/swarm/tests.rs && git commit -m "feat(app): remote-fed SwarmPane state for broker-side swarms"
```

---

### Task 7: app — route Hive actions to a companion pane; close with chat

**Files:**
- Create: `crates/crew-app/src/hivepane.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod hivepane;` alongside the other `mod` lines)
- Modify: `crates/crew-app/src/poll.rs` (~line 254: the `for action in collected_actions` match gains two arms)
- Modify: `crates/crew-app/src/app.rs` (`close_pane`, ~line 103: closing a Chat pane also closes the hive pane)
- Test: `#[cfg(test)] mod tests` in `hivepane.rs`

**Interfaces:**
- Consumes: Task 5's `HostAction::{HivePlan, Hive}`, Task 6's `for_remote`/`apply_remote`, `Pane`/`PaneContent` (`pane.rs`), `PLACEHOLDER_RECT` + `FALLBACK_SIZE` (`spawn.rs`/`app.rs`) — mirror the `Pane` literal in `spawn_labeled_terminal_in` (`spawn.rs:81-97`) and the swarm-pane construction in `spawn_goal_pane` (find it via `rg -n "spawn_goal_pane" crates/crew-app/src`).
- Produces:
  - `CrewApp::hive_plan(&mut self, tasks: Vec<crew_hive::TaskSpec>)` — open or refresh the pane labeled `"hive"`.
  - `CrewApp::hive_event(&mut self, event: &crew_hive::HiveEvent)` — apply to that pane if present.

- [ ] **Step 1: Write the failing test** (in `hivepane.rs`)

```rust
#[cfg(test)]
mod tests {
    use crate::app::CrewApp;
    use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};

    fn plan() -> Vec<TaskSpec> {
        vec![TaskSpec {
            id: TaskId(0),
            title: "t0".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
        }]
    }

    #[test]
    fn hive_plan_opens_one_companion_pane_and_reuses_it() {
        let mut app = CrewApp::default();
        app.hive_plan(plan());
        assert_eq!(app.panes.len(), 1);
        assert_eq!(app.panes[0].label.as_deref(), Some("hive"));
        // A second run reuses the pane instead of stacking a new one.
        app.hive_plan(plan());
        assert_eq!(app.panes.len(), 1);
    }

    #[test]
    fn hive_event_without_pane_is_ignored() {
        let mut app = CrewApp::default();
        // Must not panic or create panes.
        app.hive_event(&crew_hive::HiveEvent::TaskStateChanged {
            task: TaskId(0),
            state: crew_hive::TaskState::Running,
        });
        assert!(app.panes.is_empty());
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app --bin crew hivepane 2>&1 | tail -5`
Expected: COMPILE FAIL — methods missing.

- [ ] **Step 3: Implement `hivepane.rs`**

```rust
//! Companion swarm-graph pane for broker-side /crew swarms: HivePlan opens
//! (or refreshes) one pane labeled "hive"; Hive events feed its fleet.
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;
use crate::swarmpane::SwarmPane;

/// The label identifying the single companion pane. Lookup is by label so
/// pane-index churn (close/reorder) can never orphan it.
const HIVE_LABEL: &str = "hive";

impl CrewApp {
    fn hive_pane_idx(&self) -> Option<usize> {
        self.panes.iter().position(|p| {
            p.label.as_deref() == Some(HIVE_LABEL)
                && matches!(p.content, PaneContent::Swarm(_))
        })
    }

    /// A swarm plan landed: open the companion pane, or reset an existing one
    /// to the new run's graph. Never steals focus — the chat pane is primary.
    pub(crate) fn hive_plan(&mut self, tasks: Vec<crew_hive::TaskSpec>) {
        let pane = match SwarmPane::for_remote(tasks) {
            Ok(p) => p,
            Err(e) => {
                self.set_status(format!("swarm plan invalid: {e}"));
                return;
            }
        };
        match self.hive_pane_idx() {
            Some(i) => {
                self.panes[i].content = PaneContent::Swarm(Box::new(pane));
            }
            None => {
                self.panes.push(Pane {
                    content: PaneContent::Swarm(Box::new(pane)),
                    grid: crate::app::FALLBACK_SIZE,
                    rect: PLACEHOLDER_RECT,
                    label: Some(HIVE_LABEL.to_string()),
                    name: None,
                    dir: None,
                    activity: false,
                    bell: false,
                    hidden: false,
                    attention: None,
                });
            }
        }
        self.redraw();
    }

    /// Forwarded telemetry for the companion pane; ignored when absent.
    pub(crate) fn hive_event(&mut self, event: &crew_hive::HiveEvent) {
        if let Some(i) = self.hive_pane_idx() {
            if let PaneContent::Swarm(s) = &mut self.panes[i].content {
                if s.apply_remote(event) {
                    self.redraw();
                }
            }
        }
    }
}
```

(Adapt the `Pane` literal and `PaneContent::Swarm` boxing to the real definitions in `pane.rs` — if `PaneContent::Swarm` holds an unboxed `SwarmPane`, drop the `Box::new`. Mirror `spawn_goal_pane` exactly.)

- [ ] **Step 4: Wire the actions** in `poll.rs` (~line 254):

```rust
HostAction::SpawnPane { command, args, label } => {
    self.spawn_labeled_terminal(&command, &args, label)
}
HostAction::SendPane { label, text } => self.send_to_label(&label, &text),
HostAction::HivePlan { tasks } => self.hive_plan(tasks),
HostAction::Hive { event } => self.hive_event(&event),
```

- [ ] **Step 5: Close-with-chat** in `app.rs` `close_pane` — before removing index `i`, when `matches!(self.panes[i].content, PaneContent::Chat(_))`, look up `hive_pane_idx()`; if `Some(j)` with `j != i`, remove the higher index first, then the lower, then run the existing reindex logic (`grid.on_close`) for both in that order. Follow the existing body's structure; add a test in `app_tests.rs`:

```rust
#[test]
fn closing_chat_pane_closes_hive_companion() {
    let mut app = CrewApp::default();
    // A chat pane (index 0) — construct however existing chat tests do —
    // plus the companion.
    app.push_test_chat_pane(); // reuse the existing chat-test helper (see chat_tests.rs)
    app.hive_plan(vec![]);     // empty plan is a valid graph
    assert_eq!(app.panes.len(), 2);
    app.close_pane(0);
    assert!(app.panes.is_empty(), "companion closes with its chat");
}
```

(If no chat-pane test helper exists, follow how `app_tests.rs` builds panes for `reconcile_grid_*` tests and give the pane `PaneContent::Chat` the same way; if constructing a headless ChatPane is impractical, mark the companion-close coverage as a manual-verification item in the final task instead — do not fake it.)

- [ ] **Step 6: Run to verify pass**

Run: `cargo test -p crew-app --bin crew 2>&1 | tail -3`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
cargo fmt && git add crates/crew-app/src/hivepane.rs crates/crew-app/src/main.rs crates/crew-app/src/poll.rs crates/crew-app/src/app.rs crates/crew-app/src/app_tests.rs && git commit -m "feat(app): companion swarm-graph pane driven by broker Hive events"
```

---

### Task 8: full verification + smoke

**Files:** none new.

- [ ] **Step 1: Full workspace suite**

Run: `cargo test --workspace 2>&1 | grep -E "result|error" | tail -20`
Expected: all `ok`, zero `failed`.

- [ ] **Step 2: Offline smoke via the mock harness**

Run (headless broker round-trip, no GUI):

```bash
printf '%s\n' '{"type":"hello","v":1}' '{"type":"send","channel":"crew","text":"summarize the repo"}' \
  | CREW_BROKER_MOCK_REPLY="mock says hi" cargo run -q --bin crew -- --broker-plugin | head -30
```

Expected: a `ready` event, `roster`, a `hive_plan` event with tasks, `hive` telemetry events, per-task `message`s containing "mock says hi", and a final "swarm done" message. (Exact binary/flag: `main.rs:132` dispatches `--broker-plugin`; adjust if the flag differs.)

- [ ] **Step 3: GUI smoke (optional, macOS)** — use the repo's `verify` skill recipe (isolated HOME, `CREW_BROKER_MOCK_REPLY`) to type a task into `/crew` and screenshot the chat + companion pane.

- [ ] **Step 4: Final commit / merge per repo flow**

```bash
cargo fmt && git status --short   # confirm clean
# then the repo's usual no-ff merge of the feature branch into main
```
