# Streaming Token Stats (Phase B) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The roster's token column climbs from real mid-reply data: providers stream SSE chunks, the broker emits rate-limited `StatsTick` estimates, and the app retargets the Phase A count-up.

**Architecture:** `Provider` gains a default-implemented `complete_streaming(req, on_chunk)`; the OpenAI-compatible HTTP provider implements SSE (pure line parser + reqwest byte stream) and `MockProvider` chunks its reply. The broker's `Adapter` gains `call_with_usage_ticked` (default delegates); `ApiAdapter` overrides it with a chars/4 estimator. `relay_turn`/`fan_out` receive a thread-safe tick emitter (the chunk callback fires on runtime threads while the hop blocks) plus a pure `should_tick` rate limiter. The app absorbs `StatsTick` into the Phase A `RosterAnim` tok ease, gated by an open-reply set (opened on `Activity{"thinking"}`, closed on per-agent `Stats`).

**Tech Stack:** Rust workspace v0.5.57; existing deps only (`reqwest` streaming via `bytes_stream()` — the `stream` feature comes with reqwest's default surface used here; verify with `cargo build`, and if the feature is missing add `features = [.., "stream"]` to the workspace reqwest line).

**Spec:** `docs/superpowers/specs/2026-07-09-streaming-stats-design.md` (including the "Provider streaming" section added 2026-07-09)

## Global Constraints

- Wire tag is exactly `"type":"stats_tick"` with fields `agent: String`, `tokens: u64` (running OUTPUT-token estimate for the in-flight reply). No other protocol changes.
- Rate limit: at most one tick per agent per 150ms, and only when the estimate grew. Zero ticks for non-streaming providers/replies — no tick is ever required.
- The default `complete_streaming` ignores the callback and delegates to `complete`: every existing `Provider` impl must compile UNCHANGED.
- Clock discipline: rate-limit logic is a pure function taking `now` as a parameter (mirror `Tasks::attach(.., now: Instant)`); only call sites read the clock. App-side render paths unchanged.
- App unit alignment: the tok column shows live context fill — a tick retargets to `last_known_ctx + tick.tokens`; per-agent `Stats` closes the reply (ticks ignored until the agent's next `Activity{state:"thinking"}`).
- Mid-stream SSE errors propagate as `ProviderError` like the non-streaming path; no silent partial replies.
- Pre-commit runs `cargo fmt` + `cargo check`; introduce no new warnings; run `cargo fmt` before every commit.
- Where this plan says "mirror the existing signature", the implementer copies the real signature from the named file/line rather than inventing one — the shapes below show the ADDITIONS only.

---

### Task 1: `StatsTick` wire event

**Files:**
- Modify: `crates/crew-plugin/src/protocol.rs` (PluginEvent enum ~lines 24-85; tests module ~lines 87-218)

**Interfaces:**
- Produces (Tasks 5–6 rely on): `PluginEvent::StatsTick { agent: String, tokens: u64 }`, wire form `{"type":"stats_tick","agent":"coder","tokens":128}`.

- [ ] **Step 1: Write the failing round-trip tests**

Append to the `tests` module in `protocol.rs` (match the style of `per_agent_stats_carry_the_reply_latency` at ~line 155):

```rust
    #[test]
    fn stats_tick_roundtrips() {
        let ev = PluginEvent::StatsTick {
            agent: "coder".to_string(),
            tokens: 128,
        };
        let line = serde_json::to_string(&ev).unwrap();
        assert_eq!(
            line,
            r#"{"type":"stats_tick","agent":"coder","tokens":128}"#
        );
        match serde_json::from_str::<PluginEvent>(&line).unwrap() {
            PluginEvent::StatsTick { agent, tokens } => {
                assert_eq!((agent.as_str(), tokens), ("coder", 128));
            }
            _ => panic!("wrong variant"),
        }
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-plugin stats_tick`
Expected: FAIL to compile — no variant `StatsTick`.

- [ ] **Step 3: Add the variant**

In `PluginEvent`, directly after the `Stats { .. }` variant:

```rust
    /// Mid-reply progress: `agent` has produced roughly `tokens` output
    /// tokens so far in its in-flight reply. Advisory — the end-of-hop
    /// `Stats` stays authoritative and reconciles any estimate drift.
    StatsTick {
        agent: String,
        tokens: u64,
    },
```

Check `crates/crew-app/src/chat.rs` still compiles (its match has a `_ => {}` fallback at ~line 158, so no app change is needed yet — Task 6 adds the real arm).

- [ ] **Step 4: Run to verify green**

Run: `cargo test -p crew-plugin && cargo check -p crew-app`
Expected: all pass, no new warnings.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/protocol.rs
git commit -m "feat(protocol): stats_tick event — mid-reply output-token estimates"
```

---

### Task 2: `Provider::complete_streaming` default + MockProvider chunks

**Files:**
- Modify: `crates/crew-hive/src/provider/mod.rs` (the `Provider` trait)
- Modify: `crates/crew-hive/src/provider/mock.rs`

**Interfaces:**
- Produces (Tasks 3–4 rely on):
  - `pub type ChunkFn = std::sync::Arc<dyn Fn(&str) + Send + Sync>;` (in `provider/mod.rs`)
  - Trait method, default-implemented:
    ```rust
    /// Streamed completion: `on_chunk` receives each text delta as it
    /// arrives. Default ignores the callback and delegates to `complete`,
    /// so non-streaming providers work unchanged (and emit no ticks).
    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> /* same boxed future type as complete() */ {
        let _ = on_chunk;
        self.complete(req)
    }
    ```
    Mirror `complete`'s exact return type from the trait definition (a pinned boxed `Future<Output = Result<Completion, ProviderError>> + Send`).

- [ ] **Step 1: Write the failing tests**

In `crates/crew-hive/src/provider/mock.rs`'s tests module (create one if absent, matching the crate's test style):

```rust
    #[test]
    fn mock_streams_reply_in_chunks_then_completes() {
        let p = MockProvider { reply: "alpha beta gamma delta".to_string() };
        let chunks = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let sink = chunks.clone();
        let on_chunk: ChunkFn = std::sync::Arc::new(move |s: &str| {
            sink.lock().unwrap().push(s.to_string());
        });
        let req = test_request(); // reuse/adapt however mock.rs's existing tests build a CompletionRequest
        let done = futures::executor::block_on(p.complete_streaming(req, on_chunk)).unwrap();
        let got = chunks.lock().unwrap();
        assert!(got.len() >= 2, "reply arrives in at least 2 chunks: {got:?}");
        assert_eq!(got.concat(), "alpha beta gamma delta", "chunks reassemble the reply");
        assert_eq!(done.text, "alpha beta gamma delta");
    }

    #[test]
    fn default_streaming_falls_back_without_chunks() {
        // Any provider using the trait default must behave like complete().
        // MockProvider OVERRIDES it, so exercise the default through a tiny
        // local test provider that only implements `complete`.
        struct Plain;
        impl Provider for Plain {
            fn complete(&self, _req: CompletionRequest) -> /* mirror trait */ {
                Box::pin(async {
                    Ok(Completion { text: "whole".into(), input_tokens: 1, output_tokens: 1 })
                })
            }
            /* mirror any other required trait items minimally */
        }
        let ticked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let t = ticked.clone();
        let on_chunk: ChunkFn = std::sync::Arc::new(move |_| { t.fetch_add(1, std::sync::atomic::Ordering::SeqCst); });
        let done = futures::executor::block_on(Plain.complete_streaming(test_request(), on_chunk)).unwrap();
        assert_eq!(done.text, "whole");
        assert_eq!(ticked.load(std::sync::atomic::Ordering::SeqCst), 0, "default never chunks");
    }
```

Adapt `test_request()`, the `Completion` field names, and any additional required trait items to the real definitions in `provider/mod.rs` — read them first; assertions stay as written. If `futures::executor::block_on` isn't already available in crew-hive's dev-deps, use the crate's existing async-test pattern (grep `block_on` in crew-hive tests) instead of adding a dependency.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-hive mock_streams`
Expected: FAIL to compile — no `complete_streaming`, no `ChunkFn`.

- [ ] **Step 3: Implement**

- `provider/mod.rs`: add `ChunkFn` alias + the default trait method exactly as in Interfaces.
- `mock.rs`: override `complete_streaming` — split `reply` into 3 roughly-equal word groups (2 if fewer than 3 words, 1 chunk if a single word), call `on_chunk` per group in order, then return the same `Completion` as `complete` builds:

```rust
    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> /* mirror */ {
        let reply = self.reply.clone();
        let fut = self.complete(req);
        Box::pin(async move {
            let words: Vec<&str> = reply.split_whitespace().collect();
            let per = words.len().div_ceil(3).max(1);
            let mut sent = 0;
            for group in words.chunks(per) {
                // Reconstruct with the separating spaces so chunks concat to the reply.
                let mut s = group.join(" ");
                sent += group.len();
                if sent < words.len() {
                    s.push(' ');
                }
                on_chunk(&s);
            }
            fut.await
        })
    }
```

(If `self.reply` contains repeated/leading whitespace this normalizes it — acceptable for a mock; the test's reply is single-spaced.)

- [ ] **Step 4: Run to verify green**

Run: `cargo test -p crew-hive`
Expected: all pass including both new tests; no new warnings.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-hive/src/provider/mod.rs crates/crew-hive/src/provider/mock.rs
git commit -m "feat(hive): Provider::complete_streaming default + chunked MockProvider"
```

---

### Task 3: SSE streaming in the OpenAI-compatible HTTP provider

**Files:**
- Modify: `crates/crew-hive/src/provider/openai_http.rs`
- Possibly modify: root `Cargo.toml` (reqwest `stream` feature, only if the build demands it)

**Interfaces:**
- Consumes: `ChunkFn`, trait method from Task 2.
- Produces: a PURE parser used by the override and tests:
  ```rust
  pub(crate) enum SseItem {
      Delta(String),          // choices[0].delta.content
      Usage(u64, u64),        // (input_tokens/prompt_tokens, output_tokens/completion_tokens)
      Done,                   // "data: [DONE]"
      Skip,                   // keep-alives, empty lines, unparseable frames
  }
  pub(crate) fn parse_sse_line(line: &str) -> SseItem
  ```

- [ ] **Step 1: Write the failing parser tests**

Append to `openai_http.rs`'s tests module (one exists for `retry_delay` — match its style):

```rust
    #[test]
    fn sse_parser_extracts_deltas_usage_and_done() {
        assert!(matches!(parse_sse_line(""), SseItem::Skip));
        assert!(matches!(parse_sse_line(": keep-alive"), SseItem::Skip));
        assert!(matches!(parse_sse_line("data: [DONE]"), SseItem::Done));
        match parse_sse_line(r#"data: {"choices":[{"delta":{"content":"hel"}}]}"#) {
            SseItem::Delta(s) => assert_eq!(s, "hel"),
            _ => panic!("delta expected"),
        }
        // Role-only first frame: no content → Skip, not an error.
        assert!(matches!(
            parse_sse_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
            SseItem::Skip
        ));
        // Usage frame (stream_options include_usage / final frame).
        match parse_sse_line(r#"data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":42}}"#) {
            SseItem::Usage(i, o) => assert_eq!((i, o), (10, 42)),
            _ => panic!("usage expected"),
        }
        assert!(matches!(parse_sse_line("data: {not json"), SseItem::Skip));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-hive sse_parser`
Expected: FAIL to compile — `parse_sse_line`/`SseItem` not found.

- [ ] **Step 3: Implement the parser**

```rust
/// One parsed SSE line from an OpenAI-compatible streaming response.
pub(crate) enum SseItem {
    Delta(String),
    Usage(u64, u64),
    Done,
    Skip,
}

/// Pure classifier for one SSE line: `data: [DONE]`, a delta frame, a
/// usage frame, or noise (keep-alives, blanks, junk) → Skip. Never errors:
/// a malformed frame is ignored and the stream carries on.
pub(crate) fn parse_sse_line(line: &str) -> SseItem {
    let Some(data) = line.strip_prefix("data:").map(str::trim) else {
        return SseItem::Skip;
    };
    if data == "[DONE]" {
        return SseItem::Done;
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else {
        return SseItem::Skip;
    };
    if let Some(s) = v["choices"][0]["delta"]["content"].as_str() {
        if !s.is_empty() {
            return SseItem::Delta(s.to_string());
        }
    }
    if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
        let i = u["prompt_tokens"].as_u64().unwrap_or(0);
        let o = u["completion_tokens"].as_u64().unwrap_or(0);
        if i > 0 || o > 0 {
            return SseItem::Usage(i, o);
        }
    }
    SseItem::Skip
}
```

- [ ] **Step 4: Implement `complete_streaming` for the HTTP provider**

Override on the provider struct, mirroring `complete`'s request construction (same endpoint/headers/body builder — read `complete` first and REUSE its request-building code, extracting a shared helper if it's inline):

- Body additions: `"stream": true` and `"stream_options": {"include_usage": true}` (harmless where unsupported: DashScope-compatible mode accepts it; if the upstream 400s on `stream_options`, retry once without it — implement as: first attempt includes it, on a 400 status build the request again without `stream_options` before giving up).
- Send, then consume `resp.bytes_stream()` (`futures::StreamExt`); maintain a `String` carry buffer; on each bytes chunk, append lossy-utf8, split on `\n`, keep the trailing partial line in the carry; feed complete lines to `parse_sse_line`:
  - `Delta(s)` → `text.push_str(&s); on_chunk(&s);`
  - `Usage(i, o)` → remember as the authoritative usage
  - `Done` → break
  - `Skip` → continue
- Non-2xx status: map to the same `ProviderError` the non-streaming path produces (reuse its status handling — including the retry_delay integration if `complete` consults it; if that logic is inline in `complete`, factor the shared piece rather than duplicating).
- Stream transport error mid-way → return `ProviderError` (no partial success).
- Build the final `Completion` with the streamed `text` and the remembered usage; when no usage frame arrived, fall back to the same estimation `complete` uses for missing usage (read how `complete` fills usage when the response lacks it and mirror that).

- [ ] **Step 5: Run to verify green + full crate**

Run: `cargo test -p crew-hive && cargo build -p crew-hive`
Expected: all pass. If `bytes_stream` is missing, add `"stream"` to the workspace reqwest features in the root `Cargo.toml` and rebuild.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-hive/src/provider/openai_http.rs Cargo.toml Cargo.lock
git commit -m "feat(hive): SSE streaming completions for the OpenAI-compatible provider"
```

(Drop Cargo.toml/Cargo.lock from the add list if untouched.)

---

### Task 4: Broker estimator — `call_with_usage_ticked` + chars/4

**Files:**
- Create: `crates/crew-plugin/src/broker/tick.rs`
- Modify: `crates/crew-plugin/src/broker/adapter.rs` (Adapter trait, ~lines 38-62)
- Modify: `crates/crew-plugin/src/broker/apiadapter.rs` (`ApiAdapter::call_with_usage` ~lines 72-101)
- Modify: the broker module file that declares submodules (add `pub(crate) mod tick;` — find with `grep -n "mod adapter" crates/crew-plugin/src/broker/*.rs crates/crew-plugin/src/*.rs`)

**Interfaces:**
- Produces (Task 5 relies on):
  - `tick.rs`: `pub(crate) fn should_tick(last_ms: Option<u64>, now_ms: u64, min_gap_ms: u64) -> bool` and `pub(crate) const TICK_GAP_MS: u64 = 150;`
  - Adapter trait addition (default-implemented so `CliAdapter` and every other impl compile unchanged):
    ```rust
    /// Like `call_with_usage`, reporting a running OUTPUT-token estimate to
    /// `on_tokens` while the reply streams. Default: no ticks.
    fn call_with_usage_ticked(
        &self,
        /* mirror call_with_usage's exact parameters */,
        on_tokens: &(dyn Fn(u64) + Send + Sync),
    ) -> /* mirror call_with_usage's exact return type */ {
        let _ = on_tokens;
        self.call_with_usage(/* same args */)
    }
    ```

- [ ] **Step 1: Write the failing tests**

`tick.rs` (new file, tests inline):

```rust
//! Tick pacing for mid-reply StatsTick emission: a pure rate gate (clock is
//! a parameter, mirroring `Tasks::attach(.., now)`) — at most one tick per
//! agent per TICK_GAP_MS, enforced by the caller holding `last_ms`.

/// Minimum gap between two StatsTicks for one agent.
pub(crate) const TICK_GAP_MS: u64 = 150;

/// First tick always passes; later ticks pass once `min_gap_ms` elapsed.
pub(crate) fn should_tick(last_ms: Option<u64>, now_ms: u64, min_gap_ms: u64) -> bool {
    last_ms.is_none_or(|l| now_ms.saturating_sub(l) >= min_gap_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_tick_passes_then_gap_enforced() {
        assert!(should_tick(None, 0, TICK_GAP_MS));
        assert!(!should_tick(Some(1000), 1149, TICK_GAP_MS));
        assert!(should_tick(Some(1000), 1150, TICK_GAP_MS));
        assert!(should_tick(Some(1000), 999, TICK_GAP_MS) == false, "clock skew saturates, no panic");
    }
}
```

(If `Option::is_none_or` is unavailable on the workspace toolchain, use `map_or(true, ..)` — check how the codebase treats the equivalent; crew-app already uses `is_some_and`.)

In `apiadapter.rs` tests (module exists? if not, create one following crew-plugin test conventions): estimator behavior through the mock provider:

```rust
    #[test]
    fn ticked_call_reports_growing_char_estimates() {
        // MockProvider streams ~3 chunks; the estimator must report a
        // non-decreasing chars/4 sequence and the final text must match.
        let adapter = /* build an ApiAdapter over crew_hive::MockProvider
                         { reply: "one two three four five six".into() } the
                         same way roster_with does (apiadapter.rs:111-148) */;
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));
        let sink = seen.clone();
        let (text, _usage) = adapter
            .call_with_usage_ticked(/* system/prompt per real signature */, &move |t| {
                sink.lock().unwrap().push(t);
            })
            .unwrap();
        assert_eq!(text, "one two three four five six");
        let ticks = seen.lock().unwrap();
        assert!(ticks.len() >= 2, "mock streams >=2 chunks: {ticks:?}");
        assert!(ticks.windows(2).all(|w| w[0] <= w[1]), "estimates never shrink");
        let total_chars = "one two three four five six".len() as u64;
        assert_eq!(*ticks.last().unwrap(), total_chars / 4, "final estimate = chars/4");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-plugin tick && cargo test -p crew-plugin ticked_call`
Expected: FAIL to compile — no `tick` module, no `call_with_usage_ticked`.

- [ ] **Step 3: Implement**

- Add `tick.rs` as above + the `mod` declaration.
- Adapter trait: add the default method exactly per Interfaces (mirroring the real `call_with_usage` signature).
- `ApiAdapter::call_with_usage_ticked` override: same body as `call_with_usage` except it calls `provider.complete_streaming(req, on_chunk)` with:

```rust
        let chars = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter = chars.clone();
        // on_tokens is a &dyn borrow from the caller; the chunk callback runs
        // on runtime threads while this call blocks, so move an owned clone
        // of what it needs. Wrap the borrow's work in the closure directly —
        // the callback cannot outlive this function because complete_streaming
        // is awaited to completion inside it (block_on), so a scoped borrow
        // is sound; if the borrow checker disagrees with the Arc<dyn Fn>
        // signature, take `on_tokens: Arc<dyn Fn(u64) + Send + Sync>` in the
        // trait instead — keep the trait and this override consistent.
        let on_chunk: crew_hive::ChunkFn = std::sync::Arc::new(move |s: &str| {
            let total = counter.fetch_add(s.len() as u64, std::sync::atomic::Ordering::SeqCst)
                + s.len() as u64;
            on_tokens(total / 4);
        });
```

  IMPORTANT lifetime note for the implementer: `Arc<dyn Fn>` requires `'static`, so a `&dyn Fn` parameter cannot be moved into it. Resolve by making the trait method take `on_tokens: std::sync::Arc<dyn Fn(u64) + Send + Sync>` (owned, clonable) instead of a borrow — update the Interfaces shape accordingly across Task 4 AND Task 5 call sites. This is the expected resolution, not a deviation.
- Export `ChunkFn` from crew-hive's public API if not already re-exported (check `crates/crew-hive/src/lib.rs` re-exports around `Provider`).

- [ ] **Step 4: Run to verify green**

Run: `cargo test -p crew-plugin && cargo test -p crew-hive`
Expected: all pass; no new warnings.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/tick.rs crates/crew-plugin/src/broker/adapter.rs crates/crew-plugin/src/broker/apiadapter.rs crates/crew-hive/src/lib.rs
git commit -m "feat(broker): call_with_usage_ticked + chars/4 streaming estimator"
```

(Adjust the add list to files actually touched, e.g. the mod-declaration file.)

---

### Task 5: Emission — rate-limited StatsTick through relay/fan

**Files:**
- Modify: `crates/crew-plugin/src/broker/relay.rs` (`relay_turn` ~line 21, hop call site; `reply_stat` at ~103 unchanged)
- Modify: `crates/crew-plugin/src/broker/fan.rs` (`fan_out` ~line 15, per-agent call site)
- Modify: `crates/crew-plugin/src/broker/stdio.rs` (construct the thread-safe tick emitter ~lines 194-210 where relay/fan are invoked; the `emit` fn at :25 shows the Out writing pattern)

**Interfaces:**
- Consumes: `call_with_usage_ticked` + `should_tick`/`TICK_GAP_MS` (Task 4), `PluginEvent::StatsTick` (Task 1).
- Produces: `relay_turn` and `fan_out` gain a parameter `tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>`; every agent hop passes a per-hop closure to `call_with_usage_ticked` that (a) rate-limits via `should_tick` with `Instant`-derived ms and a `Mutex<Option<u64>>` last-tick cell, (b) only emits when the estimate grew (`Mutex<u64>` last value), (c) emits `PluginEvent::StatsTick { agent, tokens }` through `tick_emit`.

- [ ] **Step 1: Write the failing test**

`relay.rs` has existing tests exercising `relay_turn` with closure collectors (find them: `grep -n "mod tests" crates/crew-plugin/src/broker/relay.rs` — if relay tests live elsewhere, put this beside them and adapt the harness they already use; the assertions stay):

```rust
    #[test]
    fn relay_emits_rate_limited_stats_ticks_between_activity_and_stats() {
        // Harness: whatever existing relay tests use for agents — the mock
        // provider path streams 3 chunks, so with a 0ms gap override we
        // expect >=2 ticks; with the real 150ms gap and an instant mock,
        // exactly 1 (the first) is also acceptable — assert on ordering and
        // monotonicity rather than an exact count:
        let events = /* collected Vec<PluginEvent> via the test emit + tick_emit closures */;
        let idx = |pred: &dyn Fn(&PluginEvent) -> bool| events.iter().position(pred);
        let thinking = idx(&|e| matches!(e, PluginEvent::Activity { state, .. } if state == "thinking")).unwrap();
        let first_tick = idx(&|e| matches!(e, PluginEvent::StatsTick { .. })).expect("at least one tick");
        let stats = idx(&|e| matches!(e, PluginEvent::Stats { agent, .. } if !agent.is_empty())).unwrap();
        assert!(thinking < first_tick && first_tick < stats, "tick lands mid-hop");
        let ticks: Vec<u64> = events.iter().filter_map(|e| match e {
            PluginEvent::StatsTick { tokens, .. } => Some(*tokens),
            _ => None,
        }).collect();
        assert!(ticks.windows(2).all(|w| w[0] < w[1]), "ticks only when grown: {ticks:?}");
    }
```

The mock provider is synchronous, so all 3 chunks may land within 1ms — the 150ms gate would then allow only the FIRST tick. That is spec-correct behavior; the test must accept 1..=3 ticks (hence "at least one" + strict growth).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-plugin relay_emits`
Expected: FAIL to compile — `relay_turn` has no `tick_emit` parameter.

- [ ] **Step 3: Implement**

- `stdio.rs`: where `relay_turn`/`fan_out` are called, build once per broker loop:

```rust
    // StatsTicks fire from provider runtime threads while the hop blocks the
    // broker loop, so they need their own thread-safe writer — the per-task
    // counting `emit` closure is `&mut` and unavailable mid-hop. Ticks skip
    // the counting wrapper deliberately: they're advisory, not totals.
    let tick_out = out.clone(); // Out is the existing shared stdout handle — clone/Arc per its type
    let tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync> =
        std::sync::Arc::new(move |ev| {
            let _ = emit(&tick_out, &ev); // the stdio.rs:25 emit fn; ignore write errors like other best-effort paths
        });
```

  Read `Out`'s actual type at stdio.rs and clone/share it the way the file already does; if `emit` isn't callable from another thread as-is, wrap Out in the file's existing synchronization (writeln!+flush under its lock).
- `relay_turn`/`fan_out`: add the `tick_emit` parameter; at each agent hop replace `agent.call_with_usage(..)` with:

```rust
        let last_tick_ms = std::sync::Mutex::new(None::<u64>);
        let last_value = std::sync::Mutex::new(0u64);
        let hop_start = std::time::Instant::now();
        let agent_name = agent_name.to_string(); // whatever binding holds the hop's agent id
        let te = tick_emit.clone();
        let on_tokens: std::sync::Arc<dyn Fn(u64) + Send + Sync> = std::sync::Arc::new(move |tokens| {
            let now_ms = hop_start.elapsed().as_millis() as u64;
            let mut last = last_tick_ms.lock().unwrap();
            let mut val = last_value.lock().unwrap();
            if tokens > *val && crate::broker::tick::should_tick(*last, now_ms, crate::broker::tick::TICK_GAP_MS) {
                *last = Some(now_ms);
                *val = tokens;
                te(PluginEvent::StatsTick { agent: agent_name.clone(), tokens });
            }
        });
        let result = adapter.call_with_usage_ticked(/* same args as before */, on_tokens);
```

  (Module path for `tick` per where Task 4 declared it. Adapt binding names to the real code.)
- Update every `relay_turn`/`fan_out` caller (stdio.rs and any tests) to pass `&tick_emit` (tests can pass a collector closure).

- [ ] **Step 4: Run to verify green**

Run: `cargo test -p crew-plugin && cargo test --workspace`
Expected: all pass; no new warnings.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/relay.rs crates/crew-plugin/src/broker/fan.rs crates/crew-plugin/src/broker/stdio.rs
git commit -m "feat(broker): emit rate-limited stats_tick during streaming hops"
```

---

### Task 6: App absorption — tick retargets the tok ease

**Files:**
- Modify: `crates/crew-app/src/chat.rs` (match arm after `Stats` at ~line 130; ChatPane field)
- Modify: `crates/crew-app/src/chatflow.rs` (new `absorb_stats_tick`; open/close the reply in `absorb_activity`/`absorb_stats`)
- Test: `crates/crew-app/src/chat_tests.rs` (append)

**Interfaces:**
- Consumes: `PluginEvent::StatsTick` (Task 1), Phase A's `self.anim.set_tok(agent, now, v)` and `self.ctx` map.
- Produces: `ChatPane.tick_open: std::collections::HashSet<String>`; `pub(crate) fn absorb_stats_tick(&mut self, agent: String, tokens: u64)`.

- [ ] **Step 1: Write the failing tests**

Append to `chat_tests.rs` (same `pane()` fixture and absorb call order as the Phase A tests around lines 539-585):

```rust
#[test]
fn stats_tick_retargets_tok_while_reply_open() {
    let mut c = pane();
    c.absorb_stats(0, "planner".into(), 100, 30_000); // seed ctx ground truth
    c.absorb_activity("planner".into(), "thinking", "user".into()); // opens the reply
    c.absorb_stats_tick("planner".into(), 500);
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(
        (c.anim.tok("planner", now) - 30_500.0).abs() < 1.0,
        "tick target = last ctx + estimate"
    );
}

#[test]
fn stats_tick_ignored_when_no_reply_open() {
    let mut c = pane();
    c.absorb_stats(0, "planner".into(), 100, 30_000);
    // Reply closed by the per-agent Stats above (and never opened) — a
    // straggler tick must not move the target.
    c.absorb_stats_tick("planner".into(), 9_999);
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(
        (c.anim.tok("planner", now) - 30_000.0).abs() < 1.0,
        "stale tick ignored; Stats value stands"
    );
}

#[test]
fn per_agent_stats_closes_the_open_reply() {
    let mut c = pane();
    c.absorb_activity("planner".into(), "thinking", "user".into());
    c.absorb_stats_tick("planner".into(), 100);
    c.absorb_stats(0, "planner".into(), 50, 40_000); // closes + reconciles
    c.absorb_stats_tick("planner".into(), 20_000); // late tick from the finished reply
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(
        (c.anim.tok("planner", now) - 40_000.0).abs() < 1.0,
        "authoritative Stats wins over late ticks"
    );
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app stats_tick`
Expected: FAIL to compile — no `absorb_stats_tick`.

- [ ] **Step 3: Implement**

- `chat.rs`: add field near `anim` (initialize in the constructor(s) alongside it):

```rust
    /// Agents with an in-flight streamed reply: opened by
    /// `Activity{"thinking"}`, closed by their per-agent `Stats` — late
    /// `StatsTick`s outside the window are dropped without heuristics.
    pub(crate) tick_open: std::collections::HashSet<String>,
```

  and the match arm right after the `Stats` arm:

```rust
            PluginEvent::StatsTick { agent, tokens } => {
                self.absorb_stats_tick(agent, tokens);
            }
```

- `chatflow.rs`:
  - in `absorb_activity`'s `("thinking", false)` arm: `self.tick_open.insert(agent.clone());` (next to the existing `self.anim.flash(&agent, ..)` — before the move).
  - in `absorb_stats`'s non-empty-agent path (same block that records agent_stats): `self.tick_open.remove(&agent);` — place it BEFORE the ctx/tok retargeting so the authoritative values always land.
  - new method:

```rust
    /// Mid-reply progress: retarget the tok ease to the agent's last known
    /// context fill plus the streamed output estimate. The tok column shows
    /// live context fill, and the reply joins the next prompt — so the sum
    /// is the honest live reading. Ignored when no reply is open (stale or
    /// out-of-order ticks after the authoritative Stats).
    pub(crate) fn absorb_stats_tick(&mut self, agent: String, tokens: u64) {
        if !self.tick_open.contains(&agent) {
            return;
        }
        let base = self.ctx.get(&agent).copied().unwrap_or(0);
        self.anim
            .set_tok(&agent, crate::anim::now_ms(), (base + tokens) as f32);
    }
```

- [ ] **Step 4: Run the full suite**

Run: `cargo fmt && cargo test --workspace`
Expected: all pass; no new warnings (`cargo check -p crew-app 2>&1 | grep -c "^warning"` → 0).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/chat.rs crates/crew-app/src/chatflow.rs crates/crew-app/src/chat_tests.rs
git commit -m "feat(crew): absorb stats_tick — live token count-up from streamed replies"
```

---

### Task 7: End-to-end broker check + spec/docs touch-up

**Files:**
- Modify: `docs/CREW.md` ONLY IF it documents the wire events (check `grep -n "stats" docs/CREW.md`); add a one-line `stats_tick` mention in the same style if so, else no doc change.

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Broker-level end-to-end (no GUI)**

The broker is a stdio JSON-line process — drive it directly:

```bash
printf '%s\n' '{"type":"send","channel":"crew","text":"hello"}' \
  | CREW_BROKER_MOCK_REPLY="alpha beta gamma delta epsilon zeta" cargo run -p crew-app --bin crew -- --broker-plugin \
  | head -40
```

(Adapt the input line to the host→broker command format — read how crew-app WRITES to the broker: `grep -n "send\|serde_json::to_string" crates/crew-app/src/plugin*.rs crates/crew-plugin/src/broker/stdio.rs` shows the inbound command shape; use exactly that.)

Expected in the output stream: `ready` → `roster` → `activity(thinking)` → at least one `{"type":"stats_tick","agent":...}` → per-agent `stats` → `message`. Paste the observed line sequence into the task report.

- [ ] **Step 2: Full workspace suite + warning gate**

Run: `cargo fmt && cargo test --workspace && cargo check -p crew-app -p crew-plugin -p crew-hive 2>&1 | grep -c "^warning"`
Expected: all green, `0`.

- [ ] **Step 3: Docs (conditional) + commit**

```bash
git add docs/CREW.md   # only if changed
git commit -m "feat(crew): phase B streaming stats — end-to-end verified" --allow-empty
```

(The `--allow-empty` covers the no-doc-change case so the e2e evidence commit message lands; if docs changed, drop the flag.)
