# crew-hive as the default /crew execution engine

**Date:** 2026-07-12
**Status:** Approved

## Problem

The `/crew` chat panel executes every task through a fixed, broker-side relay
of three hard-coded agents (`planner` → `coder` → `reviewer`,
`apiadapter.rs::inbuilt_agents`), hopping sequentially via `@next`/`@done`
control lines (`engine.rs::Broker::run`). Meanwhile the repo already ships a
full swarm engine — crew-hive: LLM planner, task graph, scheduler, agent
fleet, budget governor — but it is only reachable from the separate `/goal`
pane. `/crew` cannot decompose a task into a dynamic number of parallel
agents; three fixed roles are the ceiling.

**Goal:** crew-hive becomes the default execution method for any task typed
into `/crew`. The planner decides, per task, how many agents run and what
each does. UX is hybrid: the chat conversation stays, and a companion
swarm-graph pane visualizes the live run.

## Decisions (user-approved)

- **UX:** hybrid — chat pane drives the swarm; a linked swarm-graph pane
  auto-opens alongside, showing the live task graph.
- **Default routing:** plain messages always go through crew-hive dynamic
  planning. Explicit `@agent` addressing (including `@a+b` fan-out) bypasses
  the swarm and dials that agent directly, as today. The trio hop-relay is
  retired as the default path; planner/coder/reviewer remain in the roster
  for direct dials.
- **Guardrails:** same as `/goal` — $1.00 budget cap per task run
  (`GOAL_BUDGET_MICROS_USD`), concurrency 4, budget governor cancels past
  the cap. `/stop` cancels via the existing per-task cancel flag.
- **Architecture:** Approach A — the swarm runs **inside the broker
  process**, on the existing per-task worker thread. The app renders it from
  protocol events.

## Architecture

### 1. Broker swarm driver — `crates/crew-plugin/src/broker/swarm.rs` (new)

For a plain message, the worker thread spawned by `stdio.rs::send()` runs,
on a current-thread tokio runtime (mirroring `crew-app/src/swarm/bridge.rs`):

1. **Plan:** `LlmPlanner` over the discovered provider
   (`discover::provider_and_model()` — DashScope → OpenRouter → Anthropic)
   decomposes the message into a `TaskGraph`. LLM-derived plans stay forced
   to `AgentKind::Api` (existing security invariant).
2. **Execute:** `Scheduler::new(graph, board, bus, ApiFactory, 4)`
   `.with_cancel(task_cancel_flag)` + `budget_governor(bus, $1, cancel)`,
   joined with a bus-drain task.
3. **Translate:** each `HiveEvent` maps to the existing chat protocol at the
   drain point (see §4) and is additionally forwarded raw as
   `PluginEvent::Hive`.

**Fallback:** `PlanError` → emit a notice and answer with one direct
Standard-tier `ApiAdapter` reply, so chat never dead-ends.
**Offline:** no provider → `StubPlanner` + `StubFactory`, keeping the flow
(and the `CREW_BROKER_MOCK_REPLY` GUI harness) alive without keys.

`relay_counting`/`relay_turn` remain only for `@agent`-addressed messages.

### 2. crew-hive generalization (also unblocks /goal from Anthropic-only)

- `impl Provider for Arc<dyn Provider>` (blanket impl in
  `provider/mod.rs`) so `LlmPlanner<P: Provider>` accepts the broker's
  dynamic provider.
- **Model override:** `LlmPlanner::with_model(impl Into<String>)` and
  `ApiFactory::with_model(...)`. When set, requests use that model id
  (e.g. the DashScope chain head) instead of `ModelTier::model_id()`
  (Anthropic-only ids). Tiers keep driving cost accounting.

### 3. Protocol — `PluginEvent::Hive`

New variant in `protocol.rs` carrying a serialized `HiveEvent` (crew-hive
gains `serde` derives on `HiveEvent` and its id types; the `wire` module
already speaks JSON). Existing events remain unchanged, so old panes render
chat fine even if they ignore `Hive`.

### 4. Event translation (broker-side)

| HiveEvent            | PluginEvent                                        |
| -------------------- | -------------------------------------------------- |
| plan ready           | `Message` (plan summary: task titles + tiers)      |
| `TaskStateChanged`   | `Activity { agent: task title, state }`            |
| `TokenDelta`         | `StatsTick { agent, tokens }`                      |
| `CostDelta`          | folded into turn `Stats`                           |
| `OutputChunk`/result | `Message` per completed task; final aggregate      |
| `Failed`             | `Error` (+ fallback notice when planning failed)   |

### 5. Companion graph pane (app-side)

- `ChatPane::poll()` decodes `Hive` events and forwards them through its
  `PollResult` actions.
- `SwarmPane` gains a remote-fed state (`SwarmState::Remote { fleet }`)
  that applies incoming `HiveEvent`s to its `Fleet` — same view code
  (`swarm/view.rs`), no bus.
- On the first `Hive` event of a run, the app opens (or reuses) the linked
  swarm pane next to the chat pane; it closes with the chat pane
  (existing close-pane plumbing).

## Error handling

- Budget trip / `/stop`: scheduler cancel + governor already produce
  terminal task states; the translator surfaces "run cancelled (budget/user)"
  as a chat `Message`, and the graph pane shows the final states.
- Provider errors inside a task: `HiveEvent::Failed` → `Error` event; other
  tasks continue (scheduler semantics unchanged).
- Malformed `Hive` payload app-side: ignored defensively (chat still works).

## Testing

- **crew-hive:** unit tests — blanket `Arc<dyn Provider>` impl compiles and
  delegates; model override reaches `CompletionRequest.model`.
- **broker:** with `MockProvider`/stubs — plain message emits plan summary →
  task `Activity`/`Message`s → final aggregate + `Hive` events; `@agent`
  message bypasses the swarm (dials directly); `/stop` mid-run cancels and
  reports; no-key path uses stubs.
- **app:** `chatevents` tests — `Hive` event applies to the remote `Fleet`
  and requests the companion pane; unknown `Hive` payloads ignored.
- **End-to-end:** GUI verify harness (isolated HOME, `CREW_BROKER_MOCK_REPLY`)
  — type a task in `/crew`, observe plan summary, companion pane, final
  message.

## Out of scope

- Rewiring the standalone `/goal` pane onto broker discovery (it benefits
  from §2 automatically but keeps its current backend selection).
- PTY (CLI-agent) tasks inside swarm plans (`AgentKind::Pty` stays
  planner-unreachable).
- Per-task budget configuration UI (constants first; config later).
