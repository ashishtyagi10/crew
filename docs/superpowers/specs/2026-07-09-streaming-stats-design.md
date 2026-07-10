# Streaming Token Stats (Phase B) — Design

**Date:** 2026-07-09
**Status:** Draft (implement after Phase A:
`2026-07-09-roster-animation-design.md`)

## Goal

The roster's token ticker shows *real* mid-hop progress: while an agent's
reply streams, its tok value climbs from live data instead of jumping (or
rolling) once at hop end.

## Wire protocol

New `PluginEvent` variant in `crates/crew-plugin/src/protocol.rs`:

```rust
/// Mid-reply progress: `agent` has produced roughly `tokens` output tokens
/// so far in its in-flight reply. Advisory — the end-of-hop `Stats` event
/// remains the authoritative total and reconciles any estimate drift.
StatsTick {
    agent: String,
    tokens: u64,
},
```

- serde-tagged `type: "stats_tick"`, consistent with existing snake_case.
- Compatibility: host and broker are the same binary (the broker is a
  re-exec of `crew`), so both sides version together. The mock provider and
  tests must tolerate its absence: no tick is ever *required*.

## Provider streaming (added after planning discovery)

Planning found the provider layer is non-streaming end-to-end
(`Provider::complete` returns one whole `Completion`; the broker's
`ApiAdapter::call_with_usage` blocks on it), so there is no chunk hook
without extending the provider layer. Decision (2026-07-09): add real SSE
streaming rather than timer-based estimates.

- `crew_hive::Provider` gains `complete_streaming(req, on_chunk)` where
  `on_chunk: Arc<dyn Fn(&str) + Send + Sync>` receives text deltas. The
  DEFAULT implementation ignores the callback and delegates to `complete`,
  so every existing provider compiles unchanged and non-streaming providers
  simply emit no ticks.
- The OpenAI-compatible HTTP provider overrides it: `"stream": true` SSE,
  a pure line parser (delta text / usage frame / done), deltas forwarded to
  `on_chunk`, final usage taken from the stream's usage frame when present
  (else the existing estimate fallback). Mid-stream transport/parse errors
  propagate as `ProviderError` exactly like the non-streaming path.
- `MockProvider` overrides it to send its reply in 2–3 chunks before
  returning, so the GUI harness and broker tests exercise ticks.

## Broker emission

- The broker's `Adapter` trait gains `call_with_usage_ticked(.., on_tokens)`
  (default: ignore ticks, delegate to `call_with_usage`); `ApiAdapter`
  overrides it, accumulating streamed chunk chars and reporting a chars/4
  output-token estimate to `on_tokens`.
- `relay_turn`/`fan_out` pass a per-hop tick closure that rate-limits (pure
  `should_tick(last_ms, now_ms, 150)` helper, clock passed in) and emits
  `StatsTick { agent, tokens }` through a thread-safe tick emitter built in
  `stdio.rs` from the same stdout sink — the callback fires on runtime
  threads while the hop blocks, so it cannot reuse the `&mut FnMut` emit.
- Ticks are rate-limited: at most one `StatsTick` per agent per 150ms, and
  only when the estimate grew. A reply that streams no chunks (or a
  non-streaming provider) emits no ticks.
- End-of-hop `Stats` (unchanged) reconciles: the app snaps the tok target to
  the authoritative value whether or not any ticks arrived.

## App consumption

- Unit alignment: the roster's tok column shows the agent's **live context
  fill** (`Stats.ctx`), not output tokens. A tick therefore retargets the
  Phase A `Eased` tok value to `last_known_ctx + tick.tokens` — the context
  grows as the reply streams (the reply joins the next prompt), so the
  display climbs with honest semantics and snaps to the authoritative `ctx`
  when `Stats` lands. No new UI: the Phase A count-up renderer displays it.
- Ordering: on `Stats` for an agent, its in-flight tick state resets; ticks
  are ignored for that agent until its next `Activity { state: "thinking" }`
  opens a new reply. This drops stale ticks without value-comparison
  heuristics.

## Testing

- protocol.rs round-trip test for the new variant (serialize/deserialize,
  tag string stable).
- Broker-side unit test: chunk accumulation → rate-limited tick emission
  (injected clock), no ticks for non-streaming replies.
- App-side: absorb test — tick retargets eased tok; late stale tick ignored
  after Stats.
- End-to-end: mock provider ticks drive a visible count-up in the GUI
  harness.

## Out of scope (YAGNI)

Per-chunk context (prompt-side) estimates, cost-in-dollars ticking,
provider-specific exact usage streaming (only some backends offer it; the
chars/4 estimate + end reconciliation covers all uniformly), persisting tick
history.
