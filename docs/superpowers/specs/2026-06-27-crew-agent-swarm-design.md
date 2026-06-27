# Goal: Crew as a Swarm Terminal — orchestrating hundreds-to-thousands of agents

**Date:** 2026-06-27
**Status:** North-star vision + phased roadmap
**Type:** Goal / design

---

## 1. The goal in one sentence

Turn Crew from a GPU terminal that can spawn a handful of agent panes into a
**swarm terminal**: give it one task (or a queue of many), and it intelligently
plans, spawns, and coordinates **hundreds — eventually thousands — of agents**
working in concert, with a live, sci-fi mission-control view where you can watch
the whole fleet at a glance and zoom into any single agent's work.

The terminal stops being "a place where I run an agent" and becomes
**"the command deck from which I command a fleet of agents."**

---

## 2. Why this, why now

Crew already has the hard parts of a native GPU terminal working: GPU cell
rendering, PTY panes, an auto-tiling grid, and — critically — a **plugin
orchestration protocol that can already spawn and drive panes**
(`SpawnPane`/`SendPane` in `crates/crew-plugin/src/protocol.rs`, executed via
`classify()` in `chat.rs` → `poll_panes()` in `poll.rs` → `spawn.rs`).

What exists today is a *demo* of orchestration: the reference orchestrator plugin
(`crates/crew-plugin/src/orchestrator.rs`) hardcodes "spawn agent-A and agent-B."
The mechanism is real; the brain and the scale are not. The leap from 2 panes to
1000 agents is not a rewrite of the spawn path — it is four new capabilities
layered on top:

1. **A real orchestration engine** that decomposes a goal into a task graph.
2. **A scheduler + agent pool** that can run far more agents than there are tiles.
3. **A swarm visualization** that scales past the ~6–7 readable-tile ceiling.
4. **Telemetry** so a fleet of agents is observable, not a black box.

This document defines the target and a phased path to it.

---

## 3. Design principles (the non-negotiables)

These hold for every phase. They protect Crew's identity while we scale it.

- **Native GPU terminal, not a TUI.** The swarm view renders as GPU cells/tiles,
  not an overlay. Drill-down opens a real pane in the auto-tiling grid.
- **Decouple "running" from "visible."** A thousand agents can run; only a handful
  are ever full tiles. Everything else is represented in the swarm view and
  expandable on demand. This is the single most important architectural shift.
- **Bring-your-own-provider, per agent.** Every agent in the swarm can use a
  different LLM provider/model. The orchestrator chooses models per task (cost
  tiering already lives in the project — extend it to the swarm).
- **No lock-in.** A native Rust core owns the hot path, but any external
  orchestration framework (LangGraph, Claude Agent SDK, custom Python graphs)
  must be pluggable as a sidecar over a protocol — never a hard dependency in the
  core. We are free to add dependencies where they earn their place; we are not
  free to box ourselves in.
- **Pass-through keys, fieldset-card panels, auto-tiling grid** — all existing
  guardrails still hold. The swarm is additive.
- **Observable by default.** If you can't see what 500 agents are doing, you can't
  trust them. Telemetry is a first-class feature, not an afterthought.

---

## 4. Target architecture — "The Hive"

A native Rust orchestration core, with an open bridge to external engines.

```
                         ┌─────────────────────────────────────────┐
                         │              CREW (command deck)          │
                         │                                           │
   one task / a queue ──▶│  ┌──────────┐   events   ┌────────────┐  │
                         │  │ Planner  │───────────▶ │  Swarm     │  │
                         │  │ (LLM)    │             │  View      │  │──▶ you
                         │  └────┬─────┘             │ (GPU cells)│  │
                         │       │ task graph        └─────▲──────┘  │
                         │  ┌────▼──────────────┐          │         │
                         │  │   Scheduler       │   bus    │         │
                         │  │ (tokio DAG, pool, │──────────┘         │
                         │  │  backpressure)    │                    │
                         │  └────┬──────────────┘                    │
                         │       │ dispatch                          │
                         │  ┌────▼───────────────────────────────┐   │
                         │  │          Agent Pool                 │   │
                         │  │  native agents  │  PTY agents       │   │
                         │  │  (API/transcript)│ (claude-code,    │   │
                         │  │                  │  codex, shell…)  │   │
                         │  └────┬─────────────────────┬─────────┘   │
                         │       │                      │             │
                         │  ┌────▼──────┐        ┌──────▼─────────┐   │
                         │  │ Blackboard│        │ Sidecar Bridge │   │
                         │  │ (shared   │        │ (LangGraph /   │   │
                         │  │  state /  │        │  Agent SDK /   │   │
                         │  │ artifacts)│        │  custom, JSON- │   │
                         │  └───────────┘        │  RPC sidecars) │   │
                         │                       └────────────────┘   │
                         └─────────────────────────────────────────┘
```

### 4.1 Components

**Planner.** An LLM-driven decomposer. Input: a goal (or a batch of jobs).
Output: a **task graph** (DAG) — nodes are units of work with dependencies,
assigned agent type, model tier, and a result contract. Replaces the hardcoded
logic in `orchestrator.rs`. In batch mode the "graph" is a flat fan-out; in
single-goal mode it's a dependency tree that fans out and merges upward. Same
engine, two shapes — this is the **unified substrate** the goal calls for.

**Scheduler.** The heart of scale. A `tokio`-based DAG executor that:
- runs ready nodes up to a bounded concurrency limit (the **agent pool** cap),
- enforces backpressure so 1000 queued nodes don't spawn 1000 processes at once,
- handles retries, timeouts, cancellation, and fan-in (a node waits for deps),
- schedules fairly (no agent starves — fixes today's linear-scan poll loop in
  `poll.rs` which gives no fairness guarantees).
This is the migration from `std::thread` + `mpsc` to `tokio` that the existing
terminal design doc already flagged as an open question. `tokio` is already
declared in `Cargo.toml` and unused — this is what it's for.

**Agent Pool.** Two kinds of agents, one interface:
- **PTY agents** — CLI coding agents (`claude-code`, `codex`, `gemini-cli`,
  shells) in real terminal panes. This is what works today via `spawn.rs`.
- **Native API agents** — lightweight, headless transcript agents that call an
  LLM provider directly (no PTY, no subprocess). *These are the key to scale:* a
  PTY/process per agent caps you in the low hundreds; native API agents are just
  futures + a transcript buffer, so thousands become feasible. (Phase 2 of the
  existing terminal design already names native API agents — we make them the
  default swarm worker.)

**Blackboard.** A shared state/artifact store agents read and write so results
merge upward without the fragile file/sentinel convention used today. Structured
result contracts replace "grep for a sentinel."

**Bus.** A high-throughput event stream (status, token/cost deltas, output
chunks, graph transitions) from scheduler+agents to the Swarm View. Non-blocking:
the render path never waits on agent I/O.

**Sidecar Bridge.** External engines run as separate processes speaking the
plugin protocol (extended JSON-RPC over stdio/socket — a superset of today's
`PluginCommand`/`PluginEvent`). LangGraph, Claude Agent SDK, or a bespoke Python
graph can *be* the Planner/Scheduler for a given run; Crew renders and controls
it identically to native agents. **This is how we guarantee "no future
limitation" without a Python runtime in the core.**

---

## 5. The Swarm View — sci-fi mission control

The defining experience. Past ~6–7 tiles the auto-tiling grid becomes unreadable
(confirmed in the current `layout.rs`). The swarm view is a **new pane type** that
represents the whole fleet in one GPU-rendered surface, with seamless drill-down.
It adapts to fleet size so it always looks good and always stays explorable:

- **Constellation mode (default, up to ~150 agents).** Every agent is a node in a
  living graph. Edges are task-graph dependencies. **Color = state**
  (planning / running / blocked / done / failed). **Motion/pulse = activity**
  (token throughput). The structure of the work is literally visible — you watch
  a goal decompose, fan out, and converge. Looks like a command deck.
- **Heatmap mode (auto-engages past ~150–200 agents).** A dense matrix of tiny
  status cells, one per agent — a mission-control board that pulses by state and
  load. Stays legible at thousands where nodes would be sub-pixel.
- **Drill-down, always.** Hover for a tooltip card (task, model, tokens, cost,
  last line). Click/zoom to **expand any agent into a real full tile** in the
  auto-tiling grid — the existing pane, unchanged. Collapse to return to the
  fleet. This satisfies "we should be able to expand what each agent is doing"
  at any scale.
- **Fleet HUD.** A persistent fieldset-card strip: agents live/queued/done,
  aggregate tokens/sec, total cost, ETA, failures needing attention.

The view picks its own mode by agent count, so it's the best-looking option in
every case without the user choosing.

---

## 6. What changes vs. today (capability delta)

| Capability | Today | Target |
|---|---|---|
| Spawn/drive panes | ✓ works (2 hardcoded) | ✓ scheduler-driven, N agents |
| Orchestrator brain | dummy (`orchestrator.rs`) | LLM Planner → task graph |
| Concurrency | `std::thread` + `mpsc`, no fairness | `tokio` DAG scheduler + pool |
| Agent kinds | PTY/process only | PTY **+ native API agents** |
| Scale ceiling | ~10–20 comfy, ~100 ragged | hundreds local → thousands remote |
| Visible-vs-running | coupled (every agent a tile) | **decoupled** (swarm view) |
| Many-agent UI | grid breaks past ~6–7 | constellation/heatmap + drill-down |
| Result gathering | files/sentinels (fragile) | Blackboard + result contracts |
| Telemetry | none | per-agent + fleet HUD |
| External frameworks | n/a | pluggable sidecar bridge |

---

## 7. Phased roadmap

Local-first, then remote. Each phase ships something usable and becomes its own
spec → plan → implementation cycle.

### Phase 0 — Foundations (make the core async + observable)
- Migrate the agent/poll path from `std::thread` + `mpsc` to `tokio`.
- Introduce the **event bus** and a minimal per-agent telemetry struct
  (state, tokens, cost, last-line).
- Build the **LRU grid + minimized strip** that the existing design docs
  (`2026-06-19-agent-grid-layout.md`) already specify but that isn't built —
  prerequisite for decoupling "running" from "visible."
- *Ships:* a more robust multi-pane terminal with live per-agent status.

### Phase 1 — The Planner (real orchestration, modest scale)
- Replace `orchestrator.rs` with an **LLM Planner** that emits a task graph.
- Build the **DAG scheduler + bounded agent pool** (tens of PTY agents).
- Add the **Blackboard** and structured result contracts (kill the sentinel
  convention).
- *Ships:* "give Crew a goal, it decomposes and runs ~10–30 real agents to done."

### Phase 2 — Native API agents + the Swarm View (hundreds)
- Add **native API agents** (headless transcript workers) as the default scale
  worker — this is what breaks the process-per-agent ceiling.
- Build the **Swarm View** (constellation mode + drill-down + fleet HUD).
- *Ships:* hundreds of agents on one machine, watchable as a constellation,
  any agent expandable.

### Phase 3 — Density + batch mode (thousands, locally bounded)
- **Heatmap mode** for very large fleets.
- **Batch/queue mode** UI (the "many parallel jobs" half of the unified
  substrate): point Crew at a list of jobs, watch the pool chew through them.
- Cost/model governance: planner assigns model tiers per task; budget caps.
- *Ships:* thousands of *scheduled* agents, hundreds live, on a strong machine.

### Phase 4 — Remote spill (true thousands)
- **Sidecar bridge** generalized to remote workers: the scheduler dispatches
  nodes to remote crew-worker processes/machines.
- External-engine bridge (LangGraph / Agent SDK) as a first-class Planner option.
- *Ships:* the local deck commands a distributed fleet; true thousands.

---

## 8. Open questions (to resolve per-phase, not now)

- **Planner protocol shape.** Exact task-graph schema (node = task + deps + agent
  type + model + result contract). Extends today's `PluginEvent`.
- **Native API agent transcript model.** How transcript panes render and persist;
  reuse of the chat plugin's `cells()` path.
- **Scheduler fairness/priority policy.** Round-robin vs. priority vs.
  critical-path-first for the DAG.
- **Remote worker transport** (Phase 4): socket protocol, auth, artifact sync of
  the Blackboard across machines.
- **Failure UX.** How the swarm view surfaces and lets you intervene on a failed
  or stuck node without stopping the fleet.
- **Cost ceilings.** Where budget caps live and how the planner respects them.

---

## 9. Success criteria (north-star)

You type one goal into Crew. Within seconds the swarm view blooms into a
constellation as the planner decomposes the work. Dozens, then hundreds of agents
light up — each a node, each on the right model for its job. You watch the
structure converge in real time, zoom into the one node that's blocked to see
exactly why, nudge it, zoom back out, and let the fleet finish. The terminal
feels less like a tool and more like a command deck.
