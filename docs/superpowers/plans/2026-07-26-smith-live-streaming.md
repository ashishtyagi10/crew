# /smith Live Reply Streaming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replies appear in the agent smith pane as they are generated instead of landing all at once, across every agent call path (swarm, relay, fan, tool-call follow-ups).

**Architecture:** The provider already streams SSE and hands each delta to an `on_chunk` callback; the last mile is missing in three places. `crew-hive`'s `ApiAgent` switches to `complete_streaming` and publishes a new `HiveEvent::OutputDelta` per fragment. The broker coalesces fragments through a `TextGate` (80 ms) into a new additive `PluginEvent::Delta`. The app accumulates deltas into *provisional cards held outside* `messages`, so the transcript only ever contains settled replies.

**Tech Stack:** Rust 2021 workspace (`crew-hive`, `crew-plugin`, `crew-app`), `tokio` broadcast bus, `serde`/`serde_json` tagged enums for the plugin wire protocol, `winit`/`wgpu` GPU renderer.

**Spec:** `docs/superpowers/specs/2026-07-26-smith-live-streaming-design.md`

## Global Constraints

- **200-line cap per production file.** If a file you touch would exceed it, split it (tests already live in separate `*_tests.rs` files by convention). Current sizes: `tick.rs` 63, `apiagent/mod.rs` 177, `chat.rs` 472 (already over — do not grow it; put new logic in `chatflow.rs`), `chatswarmview.rs` 306 (already over — put the tail in a NEW `chattail.rs`).
- **Nothing blocking on the winit thread.** The app polls broker events synchronously on the winit thread; every rate gate in this plan exists to protect it.
- **The invariant, everywhere:** deltas are advisory; the end-of-hop `Message` is authoritative. Never make streamed text load-bearing.
- **Tests live beside code** in `#[cfg(test)] mod tests` or a `#[path = "x_tests.rs"]` sibling, matching each file's existing convention.
- **Workspace must stay green:** `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` both clean at every commit. A pre-commit hook runs `cargo fmt --check` and `cargo check` automatically.
- **Never run a local release build** (`cargo build --release`) — disk runs low. Releases go out via tag-push → CI → in-app `/update`.

---

### Task 1: `crew-hive` — `OutputDelta` event + streaming `ApiAgent`

**Files:**
- Modify: `crates/crew-hive/src/bus/event.rs:26-29`
- Modify: `crates/crew-hive/src/apiagent/mod.rs:76-122`
- Modify (no-op arms only): `crates/crew-hive/src/telemetry/mod.rs:81`, `crates/crew-plugin/src/broker/swarmmsg.rs:54`, `crates/crew-app/src/chatswarm.rs:121-123`
- Test: `crates/crew-hive/src/apiagent/tests.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: `HiveEvent::OutputDelta { agent: AgentId, text: String }` — one streamed fragment. `HiveEvent::OutputChunk` keeps its existing meaning (the agent's COMPLETE output) and is still published exactly once per successful task.

**Background:** `ApiAgent::run` currently calls the non-streaming `provider.complete(req)` and publishes one `OutputChunk` holding the whole reply. `MockProvider::complete_streaming` (`provider/mock.rs:31-53`) splits its canned reply into 3 word groups and calls `on_chunk` per group, so this is fully testable headlessly. `EventBus` is `Clone` and `AgentId` is `Clone`, so the chunk closure can own a bus handle.

- [ ] **Step 1: Add the event variant**

In `crates/crew-hive/src/bus/event.rs`, add above `OutputChunk`:

```rust
    /// One streamed fragment of an agent's in-flight reply, published as it
    /// arrives from the provider. ADVISORY: the `OutputChunk` published when
    /// the agent finishes carries the COMPLETE output and is what the
    /// transcript keeps, so a subscriber that misses deltas — or ignores them
    /// entirely — loses liveness, never content.
    OutputDelta {
        agent: AgentId,
        text: String,
    },
```

- [ ] **Step 2: Add no-op arms so the workspace still compiles**

Adding a variant breaks three exhaustive matches that have no wildcard arm. Add to each:

`crates/crew-hive/src/telemetry/mod.rs` — alongside the other arms:
```rust
            // Fragments are liveness only; `last_line` tracks settled output.
            HiveEvent::OutputDelta { .. } => {}
```

`crates/crew-plugin/src/broker/swarmmsg.rs` — in `translate`'s match:
```rust
            // Wired to PluginEvent::Delta in a later task.
            HiveEvent::OutputDelta { .. } => vec![],
```

`crates/crew-app/src/chatswarm.rs` — extend the existing ignore list at :121-123 to `HiveEvent::CostDelta { .. } | HiveEvent::OutputChunk { .. } | HiveEvent::OutputDelta { .. } | HiveEvent::Failed { .. } => {}`

- [ ] **Step 3: Run `cargo check --workspace` to confirm it compiles**

Run: `cargo check --workspace`
Expected: clean. If another exhaustive match surfaces, add the same no-op arm there.

- [ ] **Step 4: Write the failing test**

In `crates/crew-hive/src/apiagent/tests.rs`, append:

```rust
#[tokio::test]
async fn api_agent_streams_deltas_then_one_complete_chunk() {
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    let reply = "alpha beta gamma delta epsilon zeta";
    let agent = ApiAgent::new(
        Arc::new(MockProvider {
            reply: reply.into(),
        }),
        256,
    );
    let ctx = AgentContext {
        agent: AgentId(7),
        task: spec(1),
        deps: vec![],
        bus,
    };
    let res = agent.run(ctx).await;
    assert!(res.success);

    let mut deltas: Vec<String> = Vec::new();
    let mut chunks: Vec<String> = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::OutputDelta { text, .. } => deltas.push(text),
            HiveEvent::OutputChunk { text, .. } => chunks.push(text),
            _ => {}
        }
    }
    assert!(
        deltas.len() > 1,
        "MockProvider splits into 3 groups, so the reply must arrive in pieces: {deltas:?}"
    );
    assert_eq!(
        deltas.concat(),
        reply,
        "fragments concatenate to the whole reply, losing nothing"
    );
    assert_eq!(
        chunks,
        vec![reply.to_string()],
        "exactly ONE OutputChunk, carrying the complete output"
    );
}
```

- [ ] **Step 5: Run the test to verify it fails**

Run: `cargo test -p crew-hive api_agent_streams_deltas_then_one_complete_chunk`
Expected: FAIL — `deltas.len() > 1` is false (zero deltas; `complete()` never streams).

- [ ] **Step 6: Switch `ApiAgent` to the streaming call**

In `crates/crew-hive/src/apiagent/mod.rs`, replace `match provider.complete(req).await {` (:98) with the block below. Everything after the `match` — the three publishes and both `TaskResult` arms — stays exactly as it is.

```rust
            // Fragments publish as they arrive; the completed reply still
            // publishes as one OutputChunk below, so every existing consumer
            // (telemetry's last_line, the broker's transcript Message) is
            // unchanged. A provider without streaming support falls back to
            // `complete` via the trait default and simply emits no deltas.
            let delta_bus = ctx.bus.clone();
            let delta_agent = agent_id.clone();
            let on_chunk: crate::provider::ChunkFn = Arc::new(move |s: &str| {
                delta_bus.publish(HiveEvent::OutputDelta {
                    agent: delta_agent.clone(),
                    text: s.to_string(),
                });
            });
            match provider.complete_streaming(req, on_chunk).await {
```

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test -p crew-hive api_agent_streams_deltas_then_one_complete_chunk`
Expected: PASS

- [ ] **Step 8: Run the crate's whole suite for regressions**

Run: `cargo test -p crew-hive`
Expected: PASS — in particular `api_agent_completes_and_emits`, which asserts the existing `OutputChunk`/`TokenDelta`/`CostDelta` behaviour is untouched.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-hive crates/crew-plugin/src/broker/swarmmsg.rs crates/crew-app/src/chatswarm.rs
git commit -m "feat(hive): publish OutputDelta per streamed fragment from ApiAgent"
```

---

### Task 2: `crew-plugin` — `PluginEvent::Delta` + the `TextGate`

**Files:**
- Modify: `crates/crew-plugin/src/protocol.rs:78` (new variant), `crates/crew-plugin/src/protocol.rs:113+` (tests)
- Modify: `crates/crew-plugin/src/broker/tick.rs`
- Test: in-file `#[cfg(test)] mod tests` in both

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces:
  - `PluginEvent::Delta { agent: String, text: String }` (serde tag `"delta"`)
  - `pub(crate) const TEXT_GAP_MS: u64 = 80`
  - `pub(crate) fn text_streaming_enabled() -> bool`
  - `pub(crate) struct TextGate` with `TextGate::new()` and `push(&mut self, text: &str, now_ms: u64) -> Option<String>`
  - `pub(crate) fn hop_texter(tick_emit: Arc<dyn Fn(PluginEvent) + Send + Sync>, agent: String) -> Arc<dyn Fn(&str) + Send + Sync>`

**Background:** `tick.rs` already holds `hop_ticker`, the numeric sibling — read it first (63 lines) and mirror its shape: clock passed as a parameter to a pure helper, per-hop state in the closure, `Mutex` poison recovered with `unwrap_or_else(|e| e.into_inner())`. `host.rs:35` parses events with `if let Ok(ev) = serde_json::from_str::<PluginEvent>(&line)`, so an unknown `type` is silently skipped — that is what makes this variant safe against an older app talking to a newer broker.

- [ ] **Step 1: Add the wire event**

In `crates/crew-plugin/src/protocol.rs`, add to `PluginEvent` immediately before `Message`:

```rust
    /// Mid-reply text: `agent` produced `text` since the previous Delta of
    /// this hop. ADVISORY, exactly like `StatsTick` — the end-of-hop
    /// `Message` carries the full normalized reply and REPLACES anything
    /// streamed here, so a dropped or coalesced fragment can never corrupt
    /// the transcript.
    Delta {
        agent: String,
        text: String,
    },
```

- [ ] **Step 2: Write the failing protocol test**

In `crates/crew-plugin/src/protocol.rs`'s `mod tests`:

```rust
    #[test]
    fn delta_round_trips_with_type_tag() {
        let ev = PluginEvent::Delta {
            agent: "coder".into(),
            text: "partial ".into(),
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""type":"delta""#), "got: {s}");
        match serde_json::from_str::<PluginEvent>(&s).unwrap() {
            PluginEvent::Delta { agent, text } => {
                assert_eq!((agent.as_str(), text.as_str()), ("coder", "partial "));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn unknown_event_type_fails_to_parse_so_the_host_can_skip_it() {
        // host.rs uses `if let Ok(ev) = from_str(...)`, so an Err here is how
        // an older app tolerates a newer broker's events. Pin that contract.
        assert!(serde_json::from_str::<PluginEvent>(r#"{"type":"not_a_real_event"}"#).is_err());
    }
```

- [ ] **Step 3: Run to verify they pass**

Run: `cargo test -p crew-plugin protocol`
Expected: PASS (Step 1 already added the variant; these tests pin it).

- [ ] **Step 4: Write the failing gate tests**

In `crates/crew-plugin/src/broker/tick.rs`'s `mod tests`:

```rust
    #[test]
    fn text_gate_first_push_flushes_immediately() {
        let mut g = TextGate::new();
        assert_eq!(g.push("hello", 0).as_deref(), Some("hello"));
    }

    #[test]
    fn text_gate_buffers_inside_the_gap_and_loses_no_character() {
        // This is the one way TextGate MUST differ from should_tick's numeric
        // gate: a token estimate is monotonic so a skipped tick self-heals,
        // but text is cumulative — a fragment inside the gap has to be kept.
        let mut g = TextGate::new();
        let mut flushed = String::new();
        for (frag, t) in [("a", 0u64), ("b", 10), ("c", 40), ("d", 70), ("e", 100)] {
            if let Some(p) = g.push(frag, t) {
                flushed.push_str(&p);
            }
        }
        assert_eq!(flushed, "abcde", "every character reached a flush");
    }

    #[test]
    fn text_gate_never_flushes_an_empty_payload() {
        let mut g = TextGate::new();
        assert_eq!(g.push("", 0), None);
        // …and the suppressed empty push must not consume the first-flush
        // allowance, or the next real fragment would be delayed a full gap.
        assert_eq!(g.push("x", 1).as_deref(), Some("x"));
    }

    #[test]
    fn text_gate_survives_clock_skew() {
        let mut g = TextGate::new();
        assert_eq!(g.push("a", 1000).as_deref(), Some("a"));
        assert_eq!(g.push("b", 999), None, "saturating subtraction, no panic");
    }
```

- [ ] **Step 5: Run to verify they fail**

Run: `cargo test -p crew-plugin tick`
Expected: FAIL to compile — `cannot find type TextGate in this scope`.

- [ ] **Step 6: Implement the gate**

In `crates/crew-plugin/src/broker/tick.rs`, after `should_tick`:

```rust
/// Minimum gap between two Delta flushes for one agent. Tighter than
/// `TICK_GAP_MS`: text is what the eye tracks and wants to feel continuous,
/// while a token counter reads fine at 150ms.
pub(crate) const TEXT_GAP_MS: u64 = 80;

/// Whether streamed text is forwarded at all. `CREW_STREAM_TEXT=0` turns
/// every gate into a no-op, restoring the pre-streaming behaviour for a
/// regressed run or a deterministic test.
pub(crate) fn text_streaming_enabled() -> bool {
    !matches!(std::env::var("CREW_STREAM_TEXT").as_deref(), Ok("0"))
}

/// Coalescing buffer for ONE agent's streamed text, so a provider emitting
/// per-token fragments cannot flood the host (the app polls plugin events
/// synchronously on the winit thread).
///
/// Unlike [`should_tick`]'s numeric gate, a fragment arriving inside the gap
/// is BUFFERED, never dropped: a token estimate is monotonic, so a skipped
/// tick is corrected by the next one, but text is cumulative and a dropped
/// fragment would stay missing until the settled `Message` replaced the whole
/// reply. `now_ms` is a parameter rather than a clock read, so this stays a
/// pure, testable gate — the same convention as `should_tick`.
pub(crate) struct TextGate {
    buf: String,
    last_ms: Option<u64>,
}

impl TextGate {
    pub(crate) fn new() -> Self {
        Self {
            buf: String::new(),
            last_ms: None,
        }
    }

    /// Buffer `text`, returning the payload to send as one `Delta` when the
    /// gap has elapsed (the first non-empty push always passes).
    pub(crate) fn push(&mut self, text: &str, now_ms: u64) -> Option<String> {
        self.buf.push_str(text);
        // An empty buffer must not consume the first-flush allowance, or a
        // provider's empty keep-alive frame would delay the first real text.
        if self.buf.is_empty() || !should_tick(self.last_ms, now_ms, TEXT_GAP_MS) {
            return None;
        }
        self.last_ms = Some(now_ms);
        Some(std::mem::take(&mut self.buf))
    }
}

/// Build a rate-limited `on_text` callback for one agent hop: fragments go
/// through a [`TextGate`] and each flush is emitted as a `PluginEvent::Delta`
/// via `tick_emit`. Own clock and own state per call, exactly like
/// [`hop_ticker`], so concurrent hops gate independently.
///
/// There is deliberately NO end-of-hop flush: the gate may swallow the final
/// fragment, and that is fine — deltas are advisory and the settled `Message`
/// carries the full reply milliseconds later. Skipping the flush keeps this a
/// pure function of its inputs with no lifecycle to get wrong.
pub(crate) fn hop_texter(
    tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    agent: String,
) -> std::sync::Arc<dyn Fn(&str) + Send + Sync> {
    let gate = std::sync::Mutex::new(TextGate::new());
    let hop_start = std::time::Instant::now();
    let enabled = text_streaming_enabled();
    std::sync::Arc::new(move |text: &str| {
        if !enabled {
            return;
        }
        let now_ms = hop_start.elapsed().as_millis() as u64;
        let mut g = gate.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(payload) = g.push(text, now_ms) {
            tick_emit(PluginEvent::Delta {
                agent: agent.clone(),
                text: payload,
            });
        }
    })
}
```

- [ ] **Step 7: Run to verify they pass**

Run: `cargo test -p crew-plugin tick`
Expected: PASS (4 new tests plus the existing `first_tick_passes_then_gap_enforced`).

- [ ] **Step 8: Check the file cap and lint**

Run: `wc -l crates/crew-plugin/src/broker/tick.rs && cargo clippy -p crew-plugin --all-targets -- -D warnings`
Expected: under 200 lines, clippy clean. `hop_texter` will be flagged as unused until Task 3 — if clippy objects, that is expected; wire it in Task 3 rather than adding an `#[allow]`. If the dead-code warning blocks `-D warnings`, do Task 3 in the same commit.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-plugin/src/protocol.rs crates/crew-plugin/src/broker/tick.rs
git commit -m "feat(broker): add PluginEvent::Delta and the coalescing TextGate"
```

---

### Task 3: Swarm path — `OutputDelta` → per-agent gated `Delta`

**Files:**
- Modify: `crates/crew-plugin/src/broker/swarmmsg.rs:13-17` (signature), `:54` (the no-op arm from Task 1)
- Modify: `crates/crew-plugin/src/broker/swarm.rs:190-230` (drain loop state + call site)
- Test: `crates/crew-plugin/src/broker/swarm_tests.rs`

**Interfaces:**
- Consumes: `HiveEvent::OutputDelta` (Task 1); `TextGate`, `text_streaming_enabled`, `PluginEvent::Delta` (Task 2).
- Produces: `translate(ev, specialties, agent_task, gates, now_ms) -> Vec<PluginEvent>` — the two new trailing parameters.

**Background:** `translate` is called once per bus event and holds no clock, so pacing needs a home. The drain loop already threads `&mut HashMap` state (`agent_task`) through it; a second map keyed by agent NAME gives each agent an independent gate while a parallel run (scheduler cap 4) interleaves their fragments. Without this the swarm path emits one `Delta` per SSE fragment — the per-token flood the 150 ms `StatsTick` gate exists to prevent.

- [ ] **Step 1: Write the failing test**

In `crates/crew-plugin/src/broker/swarm_tests.rs`, append (add `use` lines for `AgentId`, `TaskId`, `HiveEvent`, `HashMap`, `TextGate` if the file lacks them):

```rust
#[test]
fn output_delta_coalesces_per_agent_and_never_crosses_agents() {
    let mut specialties = HashMap::new();
    specialties.insert(TaskId(1), "planner".to_string());
    specialties.insert(TaskId(2), "coder".to_string());
    let mut agent_task = HashMap::new();
    let mut gates: HashMap<String, TextGate> = HashMap::new();

    // Spawns teach `agent_task` which task (and so which specialty) each
    // AgentId belongs to — delta naming depends on it.
    for (a, t) in [(10u64, TaskId(1)), (20, TaskId(2))] {
        translate(
            &HiveEvent::AgentSpawned {
                agent: AgentId(a),
                task: t,
            },
            &specialties,
            &mut agent_task,
            &mut gates,
            0,
        );
    }

    let d = |a: u64, t: &str| HiveEvent::OutputDelta {
        agent: AgentId(a),
        text: t.into(),
    };
    let one = |evs: &[PluginEvent]| match evs {
        [PluginEvent::Delta { agent, text }] => (agent.clone(), text.clone()),
        other => panic!("expected exactly one Delta, got {other:?}"),
    };

    // First fragment per agent flushes immediately.
    let a0 = translate(&d(10, "plan-"), &specialties, &mut agent_task, &mut gates, 0);
    let b0 = translate(&d(20, "code-"), &specialties, &mut agent_task, &mut gates, 0);
    // Inside the 80ms gap: buffered, nothing emitted.
    let a1 = translate(&d(10, "more"), &specialties, &mut agent_task, &mut gates, 10);
    // Past the gap: one Delta carrying the buffered text plus this fragment.
    let a2 = translate(&d(10, "!"), &specialties, &mut agent_task, &mut gates, 200);

    assert_eq!(one(&a0), ("planner".to_string(), "plan-".to_string()));
    assert_eq!(one(&b0), ("coder".to_string(), "code-".to_string()));
    assert!(a1.is_empty(), "a fragment inside the gap buffers, not emits");
    assert_eq!(
        one(&a2),
        ("planner".to_string(), "more!".to_string()),
        "buffered text flushes with the next fragment, and coder's text never leaks in"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-plugin output_delta_coalesces_per_agent`
Expected: FAIL to compile — `translate` takes 3 arguments, not 5.

- [ ] **Step 3: Widen `translate` and implement the arm**

In `crates/crew-plugin/src/broker/swarmmsg.rs`, change the signature and doc, and replace the Task-1 no-op arm:

```rust
/// `gates` holds one [`TextGate`] per agent NAME and `now_ms` is the run's
/// elapsed clock: `translate` sees one event at a time and has no clock of
/// its own, so mid-reply pacing has to be threaded in from the drain loop the
/// same way `agent_task` is. Per-agent gates keep a parallel run's
/// interleaved fragments from being merged into one another.
pub(super) fn translate(
    ev: &HiveEvent,
    specialties: &HashMap<TaskId, String>,
    agent_task: &mut HashMap<u64, TaskId>,
    gates: &mut HashMap<String, TextGate>,
    now_ms: u64,
) -> Vec<PluginEvent> {
```

```rust
        HiveEvent::OutputDelta { agent, text } => {
            if !super::tick::text_streaming_enabled() {
                return vec![];
            }
            let name = agent_name(agent, agent_task);
            let gate = gates.entry(name.clone()).or_insert_with(TextGate::new);
            match gate.push(text, now_ms) {
                Some(payload) => vec![PluginEvent::Delta {
                    agent: name,
                    text: payload,
                }],
                None => vec![],
            }
        }
```

Add `use super::tick::TextGate;` to the file's imports (it uses `use super::*;` — confirm `TextGate` resolves; if not, import it explicitly).

- [ ] **Step 4: Thread the state through the drain loop**

In `crates/crew-plugin/src/broker/swarm.rs`, beside the other drain-loop state (`let mut agent_task: HashMap<u64, TaskId> = HashMap::new();`):

```rust
    // One TextGate per agent name, plus the run clock they pace against —
    // `translate` has neither, so both are threaded in (see its doc comment).
    let mut gates: HashMap<String, crate::broker::tick::TextGate> = HashMap::new();
    let run_start = std::time::Instant::now();
```

Then update the call site inside the drain (`for out in translate(&ev, &specialties, &mut agent_task) {`):

```rust
                            for out in translate(
                                &ev,
                                &specialties,
                                &mut agent_task,
                                &mut gates,
                                run_start.elapsed().as_millis() as u64,
                            ) {
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p crew-plugin output_delta_coalesces_per_agent`
Expected: PASS

- [ ] **Step 6: Run the crate suite**

Run: `cargo test -p crew-plugin && cargo clippy -p crew-plugin --all-targets -- -D warnings`
Expected: PASS, clippy clean. Other `translate` call sites in tests need the two new arguments — pass `&mut HashMap::new()` and `0`.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-plugin/src/broker/swarmmsg.rs crates/crew-plugin/src/broker/swarm.rs crates/crew-plugin/src/broker/swarm_tests.rs
git commit -m "feat(broker): stream swarm replies as per-agent coalesced Deltas"
```

---

### Task 4: `HopStream` — relay, fan and tool-call paths

**Files:**
- Modify: `crates/crew-plugin/src/broker/adapter.rs:65-75`
- Modify: `crates/crew-plugin/src/broker/apiadapter.rs:153-203`
- Modify: `crates/crew-plugin/src/broker/engine.rs:130-132`, `:161-162`, `:153-155`
- Modify: `crates/crew-plugin/src/broker/fan.rs:59-63`
- Modify: `crates/crew-plugin/src/broker/toolcall.rs:186-191` (+ its doc comment at :109-120)
- Test: `crates/crew-plugin/src/broker/relay_tests.rs`, `crates/crew-plugin/src/broker/toolcall_tests.rs`

**Interfaces:**
- Consumes: `hop_ticker`, `hop_texter` (Task 2).
- Produces: `pub struct HopStream { pub on_tokens: Arc<dyn Fn(u64) + Send + Sync>, pub on_text: Arc<dyn Fn(&str) + Send + Sync> }` with `HopStream::noop()`; `Adapter::call_with_usage_ticked(&self, body: &str, timeout: Duration, stream: &HopStream)`.

**Background:** These paths already stream — `apiadapter::call_with_usage_ticked` hands the provider an `on_chunk` that counts characters and throws the text away (`:169-175`). Widening the callback to a pair is the whole change. `CliAdapter` keeps the trait's default no-op impl: a single `claude -p` invocation has no incremental stdout to forward.

- [ ] **Step 1: Write the failing test**

In `crates/crew-plugin/src/broker/relay_tests.rs`, append to the module that already asserts `StatsTick` ordering (reuse that test's harness — copy its setup verbatim, then assert on `events`):

```rust
#[test]
fn relay_hop_streams_delta_text_mid_hop() {
    // Same harness as the StatsTick ordering test above: run one relay hop
    // against the mock provider and collect every emitted PluginEvent into
    // `events`. Copy that test's setup block here verbatim.
    // <SETUP: identical to `relay_hop_ticks_between_dial_and_stats`>

    let idx = |pred: &dyn Fn(&PluginEvent) -> bool| events.iter().position(pred);
    let thinking =
        idx(&|e| matches!(e, PluginEvent::Activity { state, .. } if state == "thinking"))
            .expect("a thinking activity");
    let first_delta =
        idx(&|e| matches!(e, PluginEvent::Delta { .. })).expect("at least one Delta mid-hop");
    let stats = idx(&|e| matches!(e, PluginEvent::Stats { agent, .. } if !agent.is_empty()))
        .expect("a per-agent Stats event");
    assert!(
        thinking < first_delta && first_delta < stats,
        "the Delta lands mid-hop, not after it: {events:?}"
    );

    // The mock streams synchronously so every fragment can land inside one
    // 80ms gap — that collapses to a single Delta, which is spec-correct.
    // Assert on content, never on an exact count.
    let streamed: String = events
        .iter()
        .filter_map(|e| match e {
            PluginEvent::Delta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let settled = events
        .iter()
        .find_map(|e| match e {
            PluginEvent::Message { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("a settled reply Message");
    assert!(
        settled.contains(streamed.trim()),
        "streamed text is a prefix of the settled reply: streamed={streamed:?} settled={settled:?}"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-plugin relay_hop_streams_delta_text_mid_hop`
Expected: FAIL — `expect("at least one Delta mid-hop")` panics; no Delta is ever emitted.

- [ ] **Step 3: Define `HopStream` and widen the trait**

In `crates/crew-plugin/src/broker/adapter.rs`, above the `Adapter` trait:

```rust
/// The two live signals of one in-flight hop, passed together so each call
/// site threads one value instead of two. Both are ADVISORY — the end-of-hop
/// reply and its `Stats` stay authoritative — so a backend that cannot stream
/// simply never calls them.
#[derive(Clone)]
pub struct HopStream {
    /// Running chars/4 OUTPUT-token estimate for this hop.
    pub on_tokens: std::sync::Arc<dyn Fn(u64) + Send + Sync>,
    /// Each raw text fragment as it arrives, already agent-scoped.
    pub on_text: std::sync::Arc<dyn Fn(&str) + Send + Sync>,
}

impl HopStream {
    /// Discards both signals — for call paths that never dial an agent and
    /// for tests that don't care about liveness.
    pub fn noop() -> Self {
        Self {
            on_tokens: std::sync::Arc::new(|_| {}),
            on_text: std::sync::Arc::new(|_| {}),
        }
    }
}
```

Replace the trait method (`:67-75`):

```rust
    /// Like `call_with_usage`, also reporting this hop's live signals — a
    /// running OUTPUT-token estimate and each streamed text fragment — while
    /// the reply arrives. Default: no live signals (external CLIs return one
    /// blob and have nothing incremental to forward).
    fn call_with_usage_ticked(
        &self,
        body: &str,
        timeout: Duration,
        stream: &HopStream,
    ) -> Result<(String, Usage), String> {
        let _ = stream;
        self.call_with_usage(body, timeout)
    }
```

- [ ] **Step 4: Forward the text in `apiadapter`**

In `crates/crew-plugin/src/broker/apiadapter.rs`, change the impl's signature to take `stream: &HopStream` (drop the `on_tokens: Arc<...>` parameter), update its doc comment to say it reports tokens AND text, and replace the `on_chunk` closure (`:167-175`) with:

```rust
        let chars = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter = chars.clone();
        let on_tokens = Arc::clone(&stream.on_tokens);
        let on_text = Arc::clone(&stream.on_text);
        let on_chunk: crew_hive::ChunkFn = Arc::new(move |s: &str| {
            // Text first: it is what the user sees, and the token estimate
            // must never delay it.
            on_text(s);
            // Unicode chars, not bytes — byte counts over-report CJK ~3×
            // (same convention as the provider-side chars/4 estimators).
            let n = s.chars().count() as u64;
            let total = counter.fetch_add(n, std::sync::atomic::Ordering::SeqCst) + n;
            on_tokens(total / 4);
        });
```

- [ ] **Step 5: Update the four call sites**

`engine.rs` primary dial (:130-132):
```rust
            let stream = HopStream {
                on_tokens: hop_ticker(tick_emit.clone(), env.to.clone()),
                on_text: hop_texter(tick_emit.clone(), env.to.clone()),
            };
            let (reply, mut usage) =
                match agent.call_with_usage_ticked(&prompt, self.timeout, &stream) {
```

`engine.rs` `run_tools` hand-off (:153-155) — pass `&stream` where `&on_tokens` went:
```rust
            let reply = self.run_tools(
                agent, &prompt, reply, &mut stats, &mut usage, &env, &stream, sink,
            );
```

`engine.rs` repair dial (:161-162):
```rust
                let stream = HopStream {
                    on_tokens: hop_ticker(tick_emit.clone(), env.to.clone()),
                    on_text: hop_texter(tick_emit.clone(), env.to.clone()),
                };
                match agent.call_with_usage_ticked(&nudge, self.timeout, &stream) {
```

`fan.rs` (:59-63):
```rust
            let stream = HopStream {
                on_tokens: hop_ticker(tick_emit.clone(), name.clone()),
                on_text: hop_texter(tick_emit.clone(), name.clone()),
            };
            s.spawn(move || {
                let t0 = Instant::now();
                let res = agent.call_with_usage_ticked(&prompt, timeout, &stream);
```

`toolcall.rs` — `run_tools`'s parameter becomes `stream: &HopStream`, and the per-follow-up offset wrapper (:186-191) becomes:
```rust
            let base = tick_base;
            let ticked = HopStream {
                // Tokens need the running offset (see this fn's doc): each
                // dial restarts its own chars/4 estimate at 0, and the shared
                // gate only emits on growth.
                on_tokens: {
                    let on = Arc::clone(&stream.on_tokens);
                    Arc::new(move |t| on(base + t))
                },
                // Text needs NO offset — fragments are appended by the app,
                // not compared against a running total.
                on_text: Arc::clone(&stream.on_text),
            };
            match agent.call_with_usage_ticked(&follow, self.timeout, &ticked) {
```

Extend `run_tools`'s existing doc comment (:109-120) to note the tokens-vs-text asymmetry above.

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p crew-plugin relay_hop_streams_delta_text_mid_hop`
Expected: PASS

- [ ] **Step 7: Add the tool-call assertion**

In `crates/crew-plugin/src/broker/toolcall_tests.rs`, beside the existing `StatsTick`-per-follow-up-dial assertion (:447), add:

```rust
    assert!(
        events
            .iter()
            .any(|e| matches!(e, PluginEvent::Delta { agent, .. } if agent == "planner")),
        "the follow-up dial streams text too, not just token ticks: {events:?}"
    );
```

- [ ] **Step 8: Run the whole crate suite**

Run: `cargo test -p crew-plugin && cargo clippy -p crew-plugin --all-targets -- -D warnings`
Expected: PASS. Test call sites passing a bare `Arc<dyn Fn(u64)>` now need `&HopStream { .. }` or `&HopStream::noop()`; `apiadapter.rs:297` and `:325` are two such sites.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-plugin/src/broker
git commit -m "feat(broker): stream relay, fan and tool-call replies via HopStream"
```

---

### Task 5: `crew-app` — render over `&[&Message]` (mechanical, no behaviour change)

**Files:**
- Modify: `crates/crew-app/src/chatmsgs.rs:158`, `:217`, `:227`
- Modify: `crates/crew-app/src/chatplace.rs:145-161`
- Modify: `crates/crew-app/src/chatview.rs:119`, `:128`
- Modify: `crates/crew-app/src/chatscroll.rs:32`
- Modify: `crates/crew-app/src/chat.rs` (add `visible_messages`)
- Test: existing suites must pass unchanged

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `ChatPane::visible_messages(&self) -> Vec<&Message>`
  - `chatmsgs::card_lines(messages: &[&Message], cols: usize, now_ms: u64, view: View) -> Vec<CardLine>`
  - `chatmsgs::card_line_count(messages: &[&Message], cols: u16, view: View) -> usize`
  - `chatmsgs::message_cells(messages: &[&Message], cols: u16, rows: u16, top_row: u16, scroll: usize, view: View) -> Vec<CellView>`

**Background:** This is a pure refactor that must not change a single pixel — it exists so Task 6 can splice provisional cards into the render without cloning 500 `Message`s (each holding several `String`s) every frame. `&&Message` coerces to `&Message` at call sites via deref coercion, so bodies need almost no edits. Do this as its own commit: a reviewer can approve or reject it independently of the streaming behaviour.

- [ ] **Step 1: Add the composition helper**

In `crates/crew-app/src/chat.rs`, in `impl ChatPane`:

```rust
    /// Every card the transcript should draw this frame. ONE source, so the
    /// scroll clamp, the scrollbar, the link hit-test and the unread pill can
    /// never disagree about what is on screen — the same reason `View` is
    /// threaded through all four render entry points.
    pub(crate) fn visible_messages(&self) -> Vec<&Message> {
        self.messages.iter().collect()
    }
```

- [ ] **Step 2: Change the three `chatmsgs` signatures**

`&[Message]` → `&[&Message]` on `card_lines`, `card_line_count` and `message_cells`. Inside `card_lines`, `messages[i - 1].meta` and `messages.get(i + 1)` still work through auto-deref; if the compiler objects on the `get` line, use `.is_some_and(|n| tid == crate::chattime::task_tag(&n.meta))` unchanged — `n` is `&&Message` and derefs.

- [ ] **Step 3: Update the four call sites**

- `chatplace.rs:145-161` — build `let visible = pane.visible_messages();` at the top, change the early return to `if cols == 0 || rows == 0 || visible.is_empty()`, and pass `&visible` to `card_lines`.
- `chatview.rs:119` and `:128` — build `let visible = pane.visible_messages();` once in that branch and pass `&visible` to both `message_cells` and `card_line_count`.
- `chatscroll.rs:32` — `let visible = self.visible_messages();` then `card_line_count(&visible, cols, view)`.

Leave `chatlayout::wrapped_line_count(&self.messages, cols)` (`chatscroll.rs:26`) and `layout_cells(&pane.messages, ...)` (`chatview.rs:43`) alone: those are the too-short-pane plain fallback, which deliberately shows only settled text.

- [ ] **Step 4: Run the app suite**

Run: `cargo test -p crew-app`
Expected: PASS with **zero test edits to assertions**. Test call sites need a `let refs: Vec<&Message> = msgs.iter().collect();` line and `&refs` — that is the only permitted change. If any assertion needs adjusting, the refactor changed behaviour: stop and find out why.

- [ ] **Step 5: Lint**

Run: `cargo clippy -p crew-app --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src
git commit -m "refactor(chat): render the transcript over &[&Message]"
```

---

### Task 6: `crew-app` — provisional cards from `Delta`

**Files:**
- Modify: `crates/crew-app/src/chat.rs` (field, init, `poll` arms — keep this file from growing; logic goes in `chatflow.rs`)
- Modify: `crates/crew-app/src/chatflow.rs` (`absorb_delta`, `settle_stream`, `stream_key`, `flush_active_hops`)
- Test: `crates/crew-app/src/chat_tests.rs`

**Interfaces:**
- Consumes: `PluginEvent::Delta` (Task 2), `ChatPane::visible_messages` (Task 5).
- Produces: `ChatPane::streaming: Vec<Message>`; `absorb_delta(&mut self, agent: String, text: String)`; `settle_stream(&mut self, sender: &str)`.

**Background:** `chatflow::note_reply` (:87-94) already splits a sender on `" → "` to recover the bare agent name — that exact rule is what lets a relay's `"coder → user"` Message settle a card opened by a bare `"coder"` delta. Factor it into `stream_key` and have `note_reply` use it too.

- [ ] **Step 1: Write the failing tests**

In `crates/crew-app/src/chat_tests.rs`:

```rust
#[test]
fn delta_opens_a_provisional_card_then_appends_to_it() {
    let mut p = pane();
    p.absorb_delta("coder".into(), "Hello".into());
    p.absorb_delta("coder".into(), ", world".into());
    assert!(p.messages.is_empty(), "nothing settled reaches the transcript");
    assert_eq!(p.streaming.len(), 1, "one card per agent, not per delta");
    assert_eq!(p.streaming[0].text, "Hello, world");
    let visible = p.visible_messages();
    assert_eq!(visible.len(), 1, "the provisional card is drawn");
    assert_eq!(visible[0].text, "Hello, world");
}

#[test]
fn settled_message_replaces_the_provisional_card() {
    let mut p = pane();
    p.absorb_delta("coder".into(), "Hel".into());
    p.settle_stream("coder \u{2192} user");
    p.push_capped(Message {
        sender: "coder \u{2192} user".into(),
        text: "Hello, world".into(),
        ts: "1".into(),
        meta: String::new(),
    });
    assert!(p.streaming.is_empty(), "a relay sender settles the bare-name card");
    assert_eq!(p.messages.len(), 1, "exactly one card, not the stream plus the reply");
    assert_eq!(p.visible_messages().len(), 1);
    assert_eq!(p.messages[0].text, "Hello, world");
}

#[test]
fn two_agents_stream_into_separate_cards() {
    let mut p = pane();
    p.absorb_delta("planner".into(), "plan".into());
    p.absorb_delta("coder".into(), "code".into());
    p.absorb_delta("planner".into(), "ning".into());
    assert_eq!(p.streaming.len(), 2);
    let texts: Vec<&str> = p.streaming.iter().map(|m| m.text.as_str()).collect();
    assert!(texts.contains(&"planning") && texts.contains(&"code"), "{texts:?}");
}

#[test]
fn provisional_card_never_bumps_the_unread_pill() {
    let mut p = pane();
    p.scroll = 5; // scrolled up: a settled reply WOULD count as unread
    p.absorb_delta("coder".into(), "text".into());
    assert_eq!(p.unread, 0, "only settled replies are 'new'");
}

#[test]
fn turn_end_clears_a_stranded_provisional_card() {
    let mut p = pane();
    p.absorb_delta("coder".into(), "half a reply".into());
    // The empty-agent idle: the turn is over and no Message ever arrived.
    p.absorb_activity(String::new(), "idle", String::new());
    assert!(p.streaming.is_empty(), "an interrupted hop leaves no stranded card");
}

#[test]
fn export_never_contains_a_provisional_card() {
    let mut p = pane();
    p.push_capped(Message {
        sender: "coder".into(),
        text: "SETTLED".into(),
        ts: "1".into(),
        meta: String::new(),
    });
    p.absorb_delta("coder".into(), "HALFWRITTEN".into());
    let md = crate::chatexport::transcript_markdown("c", &p.messages, &chrono::Local::now());
    assert!(md.contains("SETTLED"));
    assert!(
        !md.contains("HALFWRITTEN"),
        "provisional cards live outside `messages`, so export cannot see them"
    );
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-app streaming delta_opens settled_message provisional`
Expected: FAIL to compile — no field `streaming`, no method `absorb_delta`.

- [ ] **Step 3: Add the field**

In `crates/crew-app/src/chat.rs`, in the struct:

```rust
    /// Agents mid-reply: one provisional card each, accumulating `Delta` text
    /// (most recently updated LAST — `absorb_delta` moves the card it touches
    /// to the end, which is how the overflow tail finds the newest without a
    /// timestamp field).
    ///
    /// NEVER stored in `messages`: the transcript holds only settled replies,
    /// so `/export`, `/restore`, the session log and the swarm fold are
    /// correct here by construction instead of each needing its own filter.
    pub(crate) streaming: Vec<Message>,
```

and `streaming: Vec::new(),` in `ChatPane::new`.

- [ ] **Step 4: Implement the flow logic**

In `crates/crew-app/src/chatflow.rs`, add at module level:

```rust
/// The agent name a sender or delta is keyed by: the part before a relay's
/// ` → `, so `"coder → user"` and a bare `"coder"` name the same agent.
pub(crate) fn stream_key(sender: &str) -> &str {
    sender.split(" \u{2192} ").next().unwrap_or(sender)
}
```

and in `impl ChatPane`:

```rust
    /// One streamed fragment landed: append it to that agent's provisional
    /// card, opening one on the hop's first delta. The touched card moves to
    /// the END of `streaming`, so the last entry is always the most recently
    /// updated one. Advisory — `settle_stream` discards whatever accumulated.
    pub(crate) fn absorb_delta(&mut self, agent: String, text: String) {
        if let Some(i) = self
            .streaming
            .iter()
            .position(|m| stream_key(&m.sender) == agent)
        {
            let mut card = self.streaming.remove(i);
            card.text.push_str(&text);
            self.streaming.push(card);
            return;
        }
        self.streaming.push(Message {
            sender: agent,
            text,
            ts: crate::chattime::unix_now_ms().to_string(),
            meta: String::new(),
        });
    }

    /// A settled reply arrived from `sender`: drop that agent's provisional
    /// card so the real `Message` takes its place. Any fragment the broker's
    /// gate swallowed is healed by the replacement.
    pub(crate) fn settle_stream(&mut self, sender: &str) {
        let name = stream_key(sender);
        self.streaming.retain(|m| stream_key(&m.sender) != name);
    }
```

Rewrite `note_reply`'s first line to reuse the helper (`let name = stream_key(sender);`) and add one line to `flush_active_hops`, which is the single choke point for both turn-end and broker-death:

```rust
        // A hop that never produced a Message must not strand its card.
        self.streaming.clear();
```

- [ ] **Step 5: Wire the poll arms**

In `crates/crew-app/src/chat.rs`'s `poll`, replace the `StatsTick` no-op arm's neighbourhood by adding:

```rust
                    PluginEvent::Delta { agent, text } => self.absorb_delta(agent, text),
```

and as the FIRST line of the `PluginEvent::Message` arm's body, before `self.awaiting = false;`:

```rust
                        self.settle_stream(&sender);
```

- [ ] **Step 6: Splice provisional cards into the render**

In `crates/crew-app/src/chat.rs`, change `visible_messages`'s body to:

```rust
        self.messages.iter().chain(self.streaming.iter()).collect()
```

Provisional cards therefore draw BELOW every settled message: history above, live region at the bottom next to the spinner.

- [ ] **Step 7: Run to verify they pass**

Run: `cargo test -p crew-app`
Expected: PASS, all six new tests plus the existing suite.

- [ ] **Step 8: Lint and check the file cap**

Run: `wc -l crates/crew-app/src/chatflow.rs crates/crew-app/src/chat.rs && cargo clippy -p crew-app --all-targets -- -D warnings`
Expected: clippy clean. `chatflow.rs` must stay under 200 lines; `chat.rs` was already 472 and must not have grown by more than the field, its init and the two poll lines.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-app/src
git commit -m "feat(chat): stream replies into provisional cards held outside the transcript"
```

---

### Task 7: `crew-app` — the dimmed overflow tail

**Files:**
- Create: `crates/crew-app/src/chattail.rs`
- Create: `crates/crew-app/src/chattail_tests.rs`
- Modify: `crates/crew-app/src/main.rs` or `lib.rs` (add `mod chattail;` beside the other `chat*` modules)
- Modify: `crates/crew-app/src/chatplace.rs:90-129` (`Grants` + `grants`)
- Modify: `crates/crew-app/src/chatview.rs:81-112` (draw site)

**Interfaces:**
- Consumes: `ChatPane::streaming` (Task 6).
- Produces: `chattail::tail_rows(pane: &ChatPane, cols: u16) -> u16`; `chattail::tail_cells(pane: &ChatPane, cols: u16, start_row: u16) -> Vec<CellView>`; `Grants.tail: u16`.

**Background:** The tail is an OVERFLOW view, not a second copy — it appears only when the growing card cannot be seen. `chatswarmview.rs` is already 306 lines (over the cap), so this goes in a new file. Follow `chatswarmview`'s pattern exactly: a `*_rows` function saying what the surface WANTS, `grants` deciding what it GETS, and a `*_cells` function drawing into the rows it was given — that split is why what is budgeted and what is drawn cannot disagree.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/chattail_tests.rs`:

```rust
use super::*;
use crate::chatlayout::Message;

fn streaming_pane(n: usize) -> crate::chat::ChatPane {
    let mut p = crate::chat_tests::pane();
    for i in 0..n {
        p.absorb_delta(format!("agent{i}"), "some streamed text".into());
    }
    p
}

#[test]
fn no_tail_when_the_single_growing_card_is_visible() {
    let p = streaming_pane(1); // scroll == 0, one agent → the card is on screen
    assert_eq!(tail_rows(&p, 80), 0);
}

#[test]
fn tail_appears_when_scrolled_away_from_the_live_bottom() {
    let mut p = streaming_pane(1);
    p.scroll = 5;
    assert_eq!(tail_rows(&p, 80), TAIL_ROWS);
}

#[test]
fn tail_appears_when_several_agents_stream_at_once() {
    let p = streaming_pane(3); // the newest is not the one drawing the eye
    assert_eq!(tail_rows(&p, 80), TAIL_ROWS);
}

#[test]
fn no_tail_without_any_streaming_card() {
    let mut p = crate::chat_tests::pane();
    p.scroll = 5;
    assert_eq!(tail_rows(&p, 80), 0);
}

#[test]
fn tail_follows_the_most_recently_updated_agent() {
    let mut p = streaming_pane(2);
    p.scroll = 5;
    p.absorb_delta("agent0".into(), " NEWEST".into());
    let cells = tail_cells(&p, 80, 0);
    let drawn: String = cells.iter().map(|c| c.ch).collect();
    assert!(
        drawn.contains("NEWEST"),
        "the tail tracks the last agent to produce text, not the first to start: {drawn:?}"
    );
}
```

If `chat_tests::pane()` is private, make it `pub(crate)`. Confirm `CellView`'s character field name (`ch` above) against `crew_render::CellView` and fix the test to match.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-app chattail`
Expected: FAIL — module `chattail` does not exist.

- [ ] **Step 3: Implement the tail**

Create `crates/crew-app/src/chattail.rs`:

```rust
//! The dimmed streaming tail: the last couple of rows of whatever an agent is
//! producing RIGHT NOW, drawn just above the live-run status line.
//!
//! It is an OVERFLOW view, not a second copy of the transcript. A growing
//! provisional card (`ChatPane::streaming`) is normally visible at the bottom
//! of the message area, and duplicating it there would be noise — so the tail
//! only appears when that card cannot be seen: the user has scrolled up, or
//! several agents are streaming at once so the newest is not the one drawing
//! the eye.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// Rows the tail claims when it shows at all.
pub(crate) const TAIL_ROWS: u16 = 2;

/// The card the tail mirrors: the most recently updated one. `absorb_delta`
/// moves the card it touches to the end of `streaming`, so "last" is "newest"
/// without carrying a timestamp.
fn newest(pane: &ChatPane) -> Option<&Message> {
    pane.streaming.last()
}

/// What the tail WANTS: nothing unless a streaming card exists that the user
/// cannot already see. `grants` decides what it actually gets.
pub(crate) fn tail_rows(pane: &ChatPane, _cols: u16) -> u16 {
    let hidden = pane.scroll > 0 || pane.streaming.len() > 1;
    if hidden && newest(pane).is_some() {
        TAIL_ROWS
    } else {
        0
    }
}

/// Draw the tail into `TAIL_ROWS` rows starting at `start_row`: the last rows
/// of the newest card's text, wrapped to `cols` and muted.
pub(crate) fn tail_cells(pane: &ChatPane, cols: u16, start_row: u16) -> Vec<CellView> {
    let Some(card) = newest(pane) else {
        return Vec::new();
    };
    if cols == 0 {
        return Vec::new();
    }
    let muted = crew_theme::theme().text_muted;
    let page = crew_theme::theme().page_bg;
    // Reuse the transcript's own wrapper so the tail breaks text exactly the
    // way the card above it does.
    let chars: Vec<char> = card.text.chars().collect();
    let wrapped = crate::chatlayout::wrap_indices(&chars, cols as usize);
    let last: Vec<(usize, usize)> = wrapped
        .iter()
        .rev()
        .take(TAIL_ROWS as usize)
        .rev()
        .copied()
        .collect();
    let mut out = Vec::new();
    for (i, (s, e)) in last.iter().enumerate() {
        let row = start_row + i as u16;
        let line: String = chars[*s..*e].iter().collect();
        out.extend(crate::chatwidth::place_row(&line, row, cols, muted, page));
    }
    out
}

#[cfg(test)]
#[path = "chattail_tests.rs"]
mod tests;
```

Check `chatwidth::place_row`'s real signature before writing this call and adapt — `chatswarmview` uses it and is the reference. If its parameters differ, match that usage rather than the sketch above.

- [ ] **Step 4: Register the module**

Add `mod chattail;` beside the other `chat*` module declarations (in `main.rs`, sorted with its neighbours).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p crew-app chattail`
Expected: PASS

- [ ] **Step 6: Budget the rows**

In `crates/crew-app/src/chatplace.rs`, add `pub tail: u16,` to `Grants` (documented as "the dimmed streaming overflow tail (0 or `TAIL_ROWS`)"), and in `grants` add the take AFTER `prog` — the tail is the most expendable surface, since the text it mirrors also exists in the transcript:

```rust
    let tail = take(crate::chattail::tail_rows(pane, cols));
```

and `tail,` in the returned struct.

- [ ] **Step 7: Draw it**

In `crates/crew-app/src/chatview.rs`'s non-empty branch, after the swarm block is drawn, stack the tail directly above it:

```rust
        // Above the live status line: the streaming overflow tail, when
        // `grants` could seat it (0 rows means it was not budgeted, so it is
        // skipped entirely rather than sharing another surface's row).
        if g.tail > 0 {
            let tail_start = rows
                .saturating_sub(bottom + prog_rows + queued_rows + g.swarm + g.tail);
            cells.extend(crate::chattail::tail_cells(pane, cols, tail_start));
        }
```

- [ ] **Step 8: Run the app suite and lint**

Run: `cargo test -p crew-app && cargo clippy -p crew-app --all-targets -- -D warnings`
Expected: PASS, clean. `chatplace_tests.rs` may assert exact `Grants` fields — update those to include `tail`.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-app/src
git commit -m "feat(chat): dimmed overflow tail for off-screen streaming replies"
```

---

### Task 8: Live verification and release

**Files:**
- Modify: `Cargo.toml` (workspace version bump)
- Modify: `.superpowers/sdd/progress.md` (ledger entry)

**Interfaces:**
- Consumes: everything above.

**Background:** Unit tests cannot cover the real risk here — frame pacing and text jitter under a live provider. Per this project's release rule: **never build a release locally** (disk); tag-push → CI → in-app `/update`. The live app spawns the INSTALLED broker at `~/.local/bin/crew`, not the dev build, so a dev-build-only change will not show up until the release lands.

- [ ] **Step 1: Full workspace green**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, zero warnings.

- [ ] **Step 2: Verify in the live GUI**

Invoke the `verify` skill and follow its recipe (isolated HOME, warm relaunch, frontmost-PID guard before every synthetic keystroke, `screencapture`). Drive a real `/smith` message and confirm by screenshot:
1. Reply text appears progressively, not all at once.
2. When the hop ends, there is exactly ONE card — no duplicate of the streamed text beside the settled reply.
3. Scrolling up mid-reply shows the dimmed tail; scrolling back to the bottom hides it.
4. A parallel run (several agents) grows several cards without interleaving one agent's text into another's.

- [ ] **Step 3: Verify the kill switch**

Run the app with `CREW_STREAM_TEXT=0` and confirm the pane behaves exactly as it did before this work — one card at the end of the hop, no deltas.

- [ ] **Step 4: Record the outcome honestly**

Append a ledger entry to `.superpowers/sdd/progress.md` describing what shipped, what was verified live versus only by unit test, and anything left open. If a verification step failed, say so there and fix it before releasing — do not record a pass that did not happen.

- [ ] **Step 5: Merge and release**

```bash
git checkout main
git merge --no-ff feat/smith-live-streaming
git branch -d feat/smith-live-streaming
```

Then bump the workspace version in the root `Cargo.toml`, commit as `chore(release): vX.Y.Z`, tag it, and push main plus the tag so CI publishes. Confirm the release job succeeded before telling the user to `/update` — a tag that lags the pushed code makes `/update` a downgrade.

---

## Self-Review

**Spec coverage** — every section maps to a task: §1 wire protocol → Task 2; §2 `crew-hive` → Task 1; §3 gate + swarm pacing → Tasks 2 and 3; §3 `HopStream` + four call sites → Task 4; §3 kill switch → Task 2 (implemented) and Task 8 (verified); §4 provisional cards + render composition → Tasks 5 and 6; §4 dimmed tail → Task 7; §6 non-goals → nothing built; §Error handling → covered by Task 6's turn-end-clears test and Task 4's settled-reply assertion; §Testing items 1–9 → Tasks 1, 2, 3, 4, 6, 7, 8 respectively.

**Placeholder scan** — one deliberate marker remains: Task 4 Step 1 says `<SETUP: identical to relay_hop_ticks_between_dial_and_stats>` rather than duplicating ~40 lines of an existing in-repo harness the implementer can read directly. Two steps also say "check the real signature before writing this call" (`chatwidth::place_row`, `CellView`'s char field) — these are instructions to read neighbouring code, not unresolved decisions.

**Type consistency** — `HopStream` field names (`on_tokens`, `on_text`) are identical in Tasks 2, 4; `TextGate::push(&str, u64) -> Option<String>` is used with the same signature in Tasks 2 and 3; `translate`'s five-parameter form matches between Tasks 3's implementation and its test; `visible_messages() -> Vec<&Message>` is defined in Task 5 and only its body changes in Task 6; `stream_key` is defined once in Task 6 and reused by `note_reply`; `TAIL_ROWS` is used consistently in Task 7's tests and implementation.
