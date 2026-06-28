# Design: `/crew` → in-process hive swarm

**Date:** 2026-06-28
**Status:** Approved (design); pending spec review

## Summary

Repoint the `/crew` command from the multi-agent **broker** (claude ⇆ codex ⇆
opencode talking to each other over stdio) to the **hive swarm**: a goal is
decomposed into a task DAG, executed over a bounded pool of native LLM agents,
and rendered live as a glyph grid (constellation/heatmap) inside a GPU pane. The
broker is retired.

This is a single, focused change: one new pane kind, one threading helper, and
the deletion of the broker subsystem.

## Motivation

The repo already contains two orchestration systems:

- **Broker** (`crew-plugin/src/broker/`) — peer-to-peer routing between AI CLI
  tools via `TO <peer>:` / `DONE` directives. This is what `/crew` opens today.
- **Hive** (`crew-hive/`) — headless goal→DAG decomposition, a tokio scheduler,
  fleet telemetry, and a `FleetView` → `render_cells()` glyph renderer. Fully
  built and tested, but not wired to any pane or command.

The hive is the intended direction (recent work added swarm-view layout, batch
mode, budget governance, and `render_cells`). `/crew` should open it. The
broker's peer-chat model is superseded by the DAG model and is removed.

## Decisions (locked)

1. **`/crew` opens the hive swarm.** Broker is retired.
2. **Pane UX: swarm-view-first.** The pane *is* the FleetView glyph grid; a
   one-row goal bar sits on the bottom interior row.
3. **Agent backend: native API now.** Tasks run via `ApiAgent`
   (`AnthropicProvider`, bring-your-own-provider). CLI-as-`RemoteAgent` is a
   deliberate follow-up, out of scope here.
4. **Runtime model: in-process tokio thread.** `crew-app` depends on
   `crew-hive`; a background thread runs the `Scheduler` and streams telemetry
   back over a channel. No subprocess, no JSON IPC, no re-exec.

## Architecture

A new pane kind, `PaneContent::Crew(CrewPane)`, spawned directly by the `/crew`
slash command. The pane renders the FleetView grid filling the fieldset card,
with a single goal-input row on the bottom interior row.

A background `std::thread` hosts a tokio runtime that runs the hive
`Scheduler`. The thread folds `EventBus` events into a `Fleet`+graph snapshot
and sends it to the pane over a channel. The winit/wgpu render loop never
blocks: each frame the pane renders from the latest snapshot it has received.

### Pane states

- **Idle** — no goal yet. The grid area shows a hint: "Enter a goal for the
  swarm…". If no provider is configured, the hint instead explains that.
- **Running** — the swarm grid animates as tasks move Pending → Ready → Running
  → Done/Failed.
- **Done / Failed** — final grid plus a one-line summary (e.g.
  `done 7 · failed 1`).

## Components

### `crew-app/src/crewpane/` (new module dir)

Split to honor the hard 200-line-per-file cap:

- `mod.rs` — `CrewPane` struct, `cells(cols, rows)` entry, re-exports.
- `state.rs` — `CrewState` (`Idle` / `Running` / `Done` / `Failed`), the latest
  `TaskGraph` + `Fleet` snapshot, the goal buffer, and the `HiveHandle`.
- `keys.rs` — key handling: printable keys edit the goal buffer; `Enter`
  launches a run; `Esc` cancels a running swarm; `Backspace` edits.
- `render.rs` — snapshot → `fleet_view(graph, fleet, cols)` →
  `render_cells(view, cols, swarm_rows)` → `CellGlyph` → `CellView`; draws the
  goal bar on the bottom interior row; draws the Idle hint and the Done/Failed
  summary.

### `crew-app/src/hiverun.rs` (new)

Owns all threading so `crew-hive` stays headless and UI-independent.

- Spawns a `std::thread` that builds a tokio runtime and runs:
  `Planner::plan(goal)` → `TaskGraph`, then
  `Scheduler::run(graph, ApiAgent factory, EventBus, Blackboard)`.
- Returns a `HiveHandle { rx, cancel }`:
  - `rx` — channel receiver of telemetry snapshots (`Fleet` + graph state).
  - `cancel` — a cancellation token; `Esc` triggers the scheduler's existing
    cascade-cancel.
- If this file approaches 200 lines, split the snapshot-folding logic into a
  submodule.

### Telemetry path

The background thread subscribes to the hive `EventBus`, folds each `HiveEvent`
into a current `Fleet` + graph snapshot, and sends it over the channel. The
existing pane poll path drains pending snapshots and flags a redraw — mirroring
how chat panes poll plugin events today, but in-process with no serialization.

### Provider configuration

`ApiAgent` uses `AnthropicProvider` (reqwest), configured from the existing
config/env (bring-your-own-provider; default to the latest, most capable
model). When no key is configured, the pane stays Idle and shows a clear
message rather than launching a doomed run. `MockProvider` is used in tests.

## Data flow

```
type goal → Enter
  → hiverun spawns thread (tokio runtime)
    → Planner::plan(goal) → TaskGraph
    → Scheduler::run(graph, ApiAgent factory, EventBus, Blackboard)
      → EventBus(HiveEvent) → fold into snapshot → channel
render loop (each frame):
  latest snapshot → fleet_view → render_cells → CellView + goal bar
Esc → cancel token → scheduler cascade-cancels
```

## Wiring changes

- `dispatch.rs:14` — `"crew"` builds a `CrewPane` directly (rewrite
  `spawn_crew_pane`).
- `pane.rs` — add a `Crew` arm to `PaneContent`, to `cells()`, and to
  `title_text()` (returns "crew").
- `crew-app/Cargo.toml` — add a dependency on `crew-hive`.
- `suggest.rs` — `/crew` stays registered (no change).
- `render.rs:118` — title card for "crew" stays (no change).

## Removal (broker retirement)

Delete:

- `crew-plugin/src/broker/` (mod, agents, registry, engine, route, stdio).
- `crew-plugin/src/lib.rs` broker re-exports (the `pub use broker::{…}` block).
- `crew-plugin/src/bin/crew-broker-plugin.rs`.
- `crew-app/src/main.rs` `--broker-plugin` branch and the
  `crew_plugin::run_broker_stdio()` call.
- `crew-app/src/chatspawn.rs` `spawn_crew_pane` (broker version) and
  `crew_broker_cmd()`.
- Broker tests.

Leave intact (still load-bearing): the `crew-plugin` crate itself, `ChatPane`,
`spawn_chat_pane`, the echo and orchestrator plugins (opened from
`chords.rs:63,67`), the `crew-echo-plugin` binary, and the plugin
host/protocol.

## Error handling

- No provider key → Idle with an explanatory message; no run launched.
- `Planner` failure → error message in the pane; stay Idle.
- Provider/network error on a task → that task renders red
  (`state_color(TaskState::Failed)`); the run continues for independent tasks.
- Task failures → counted in the Done/Failed summary line.
- Background thread panic → caught; pane shows the Failed state.

## Testing

- **`crew-hive`** — add a test exercising the run composition with
  `StubPlanner` + `StubAgent` + `MockProvider`: a goal yields a completed
  `Fleet` with the expected task count and states.
- **`crew-app`** — `CrewPane` unit tests, mirroring `chat_tests` /
  `paneview_tests`:
  - Idle render shows the hint.
  - A fabricated snapshot places swarm glyphs in the grid area and renders the
    goal bar.
  - Key handling: printable keys edit the goal; `Enter` launches (with a
    stubbed `HiveHandle`); `Esc` transitions to a cancelled/Idle state.
- **Gate** — `cargo fmt`; `cargo clippy --workspace --all-targets` warning-free;
  `cargo test --workspace` green; every `.rs` file ≤ 200 lines.

## Out of scope (follow-ups)

- CLI tools (codex / claude / opencode) as `RemoteAgent` backends in the swarm.
- Per-task drill-in / inspecting a single agent's output.
- Persisting / restoring a hive run across sessions.
