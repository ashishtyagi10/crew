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

## Broker emission

- Where the broker observes streamed chunks per agent reply (crew-hive
  provider streaming path — exact hook located during planning), it
  accumulates a chars/4 token estimate.
- Ticks are rate-limited: emit at most one `StatsTick` per agent per 150ms,
  and only when the estimate grew. A reply that streams no chunks (or a
  non-streaming provider) emits no ticks.
- End-of-hop `Stats` (unchanged) reconciles: the app snaps the tok target to
  the authoritative value whether or not any ticks arrived.
- Mock provider (`CREW_BROKER_MOCK_REPLY`) emits 2–3 synthetic ticks before
  its reply so the GUI harness and app tests can exercise the path.

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
