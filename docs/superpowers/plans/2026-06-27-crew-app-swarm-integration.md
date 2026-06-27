# crew-app Swarm Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.
>
> **Status:** Ready to execute. The headless-testable tasks (1–2) can be done now; the GPU/visual tasks (3–4) need a machine with a display to runtime-verify (`cargo run -p crew-app`) and, for live agents, `ANTHROPIC_API_KEY`.

**Goal:** Wire the completed `crew-hive` engine into the `crew-app` GPU terminal so a typed goal spawns a live swarm, rendered as the sci-fi constellation/heatmap pane with drill-down — using crew-app's existing background-thread + per-frame mpsc poll pattern (the same way it already drives PTYs and chat plugins), so no winit/tokio coexistence problem arises.

**Architecture:** The scheduler runs on a dedicated OS thread that owns a single-threaded `tokio` runtime (`Runtime::new().block_on(scheduler.run())`). Its `EventBus` is drained on that thread into a `std::sync::mpsc::Sender<HiveEvent>`; the app holds the `Receiver` and drains it each frame (mirroring `poll.rs`/`chat.rs`), applying events to a `crew_hive::Fleet`. A new `SwarmPane` holds that `Fleet` + the `TaskGraph` and, each frame, builds a `FleetView` (`fleet_view`), renders it with `render_cells`, and maps each `CellGlyph` → `CellView` inside a fieldset card (via the existing `panecard`/`PaneScene` path). A `/swarm <goal>` input command launches the engine thread (with the LLM planner + `ApiAgent`s when a key is present, or `StubPlanner` + stub agents for a keyless demo). Clicking a node drills into that agent (future: open its transcript pane).

**Tech Stack:** Rust, `crew-hive` (new crew-app dependency), `tokio` (single-thread runtime on a worker thread), `std::sync::mpsc`, existing `crew-app` render path. No new external deps (tokio is already a workspace dep; add it + crew-hive to crew-app's Cargo.toml).

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total. crew-app files are tight — split aggressively into submodules.
- **No new external dependencies** — add `crew-hive = { path = "../crew-hive" }` and `tokio = { workspace = true }` to `crates/crew-app/Cargo.toml` (both already in the workspace).
- Reuse existing patterns: pane content via `PaneContent` enum + `cells()`; rendering via `panecard`/`PaneScene`; background work via a thread + `mpsc` drained in the frame loop (see `poll.rs`, `chat.rs`).
- Do NOT break the auto-tiling grid, pass-through keys, fieldset-card panels, or the LRU strip.
- The engine thread must be cancellable (use `crew_hive` `Scheduler::with_cancel` + an `Arc<AtomicBool>`) and must not block the UI thread.

---

### Task 1: Engine bridge — run the scheduler off-thread, drain events to the app (HEADLESS-TESTABLE)

**Files:**
- Create: `crates/crew-app/src/swarm/mod.rs`
- Create: `crates/crew-app/src/swarm/bridge.rs`
- Create: `crates/crew-app/src/swarm/tests.rs`
- Modify: `crates/crew-app/Cargo.toml` (add `crew-hive`, `tokio`)
- Modify: `crates/crew-app/src/main.rs` (declare `mod swarm;`)

**Interfaces:**
- `pub struct SwarmHandle { rx: std::sync::mpsc::Receiver<crew_hive::HiveEvent>, cancel: Arc<AtomicBool>, graph: crew_hive::TaskGraph }` with:
  - `pub fn spawn(graph: TaskGraph, factory: Arc<dyn crew_hive::AgentFactory>, concurrency: usize) -> SwarmHandle` — spawns the engine thread (tokio current-thread runtime + `Scheduler::new(...).with_cancel(cancel).run()`), with a bus-drain task forwarding `HiveEvent`s to the mpsc sender; returns the handle.
  - `pub fn drain(&self, fleet: &mut crew_hive::Fleet)` — non-blocking `try_recv` loop applying events to the fleet (called each frame).
  - `pub fn cancel(&self)` — sets the cancel flag.
  - `pub fn graph(&self) -> &TaskGraph`.
- **Testable headlessly:** `spawn` with a `StubFactory` over a tiny graph, then poll `drain` until the fleet shows all tasks done — no GPU, no key. (Use a short sleep/yield loop in the test.)

- [ ] Steps: add deps; implement the bridge (thread owns `tokio::runtime::Builder::new_current_thread().enable_all().build()`, runs `block_on`; a subscriber loop forwards bus events to the std mpsc); write a `#[test]`/`#[tokio::test]`-free integration-style test that drives a `StubFactory` graph to completion via `drain`; `cargo test -p crew-app swarm::`; `cargo fmt && cargo clippy --workspace --all-targets`; commit `feat(crew): swarm engine bridge (off-thread scheduler -> frame-drained events)`.

---

### Task 2: Swarm cell view — Fleet → CellViews (HEADLESS-TESTABLE)

**Files:**
- Create: `crates/crew-app/src/swarm/view.rs`
- Modify: `crates/crew-app/src/swarm/mod.rs`, `crates/crew-app/src/swarm/tests.rs`

**Interfaces:**
- `pub fn swarm_cells(graph: &TaskGraph, fleet: &Fleet, cols: u16, rows: u16) -> Vec<crew_render::CellView>` — calls `crew_hive::fleet_view` + `crew_hive::render_cells`, maps each `CellGlyph { col, row, ch, color }` to a `CellView { col, row, c: ch, fg: color.into rgb tuple, bg: (0,0,0), .. }`. Plus a HUD line (live/done/failed/cost from `fleet.totals()`) on row 0.
- **Testable headlessly:** construct a `Fleet` from events, assert `swarm_cells` produces cells at expected positions/colors and a HUD row — no GPU.

- [ ] Steps: implement; unit-test cell positions + HUD against a constructed fleet; `cargo test -p crew-app swarm::`; fmt/clippy; commit `feat(crew): swarm_cells — Fleet -> CellViews + fleet HUD`.

---

### Task 3: SwarmPane + `/swarm` command (NEEDS GUI TO VERIFY)

**Files:**
- Create: `crates/crew-app/src/swarmpane.rs` (a `SwarmPane { handle: SwarmHandle, fleet: Fleet }` whose `cells(cols, rows)` drains + renders via `swarm_cells`)
- Modify: `crates/crew-app/src/pane.rs` (add `PaneContent::Swarm(SwarmPane)` + wire `title_text`/`cells`)
- Modify: the input/command path (`suggest.rs`/`handler.rs`/wherever `/` commands dispatch) to add `/swarm <goal>`: builds a `StubPlanner` graph (keyless demo) or `LlmPlanner` (when `ANTHROPIC_API_KEY` set), a factory (`ApiAgent` or stub), `SwarmHandle::spawn`, and pushes a `Swarm` pane.
- Modify: `poll.rs` (or the frame loop) to mark the swarm pane for redraw each frame so its animation updates.

- [ ] Steps: implement per existing pane patterns; keep every file ≤ 200 (split). Unit-test what you can (command parsing, pane `title_text`). **Runtime-verify** with `cargo run -p crew-app` → type `/swarm build X` → watch the constellation populate (stub agents, no key). Commit `feat(crew): SwarmPane + /swarm command (stub-driven demo)`.

---

### Task 4: Live agents + drill-down (NEEDS GUI + ANTHROPIC_API_KEY)

- [ ] When `ANTHROPIC_API_KEY` is present, `/swarm` uses `LlmPlanner` + `ApiAgent` (real decomposition + real agents). Runtime-verify the full sci-fi loop. Add node drill-down: clicking a node opens that agent's transcript (a new pane showing its accumulated `OutputChunk`s). Commit per sub-feature.

---

## Self-Review

- **Spec coverage:** wires the engine into the terminal and renders the constellation/heatmap with a fleet HUD and drill-down — the on-screen swarm. Tasks 1–2 are headless-testable now; 3–4 need a display (and 4 a key). ✅
- **Architecture fit:** background thread + mpsc frame-drain matches crew-app's existing PTY/plugin pattern — no winit/tokio coexistence issue. ✅
- **No new external deps; guardrails respected** (auto-tiling, fieldset cards, pass-through keys, LRU strip untouched). ✅

## Notes for the executor

- crew-app currently uses `std::thread` + `mpsc` and has **no** tokio runtime on the UI thread — keep it that way; tokio lives only on the engine worker thread.
- `Rgb(u8,u8,u8)` → `CellView.fg` is a `(u8,u8,u8)` tuple; trivial map.
- For the keyless demo, `StubAgent`/`StubFactory` from crew-hive let the whole pane animate with zero external calls — ideal for a first GPU smoke test before wiring live agents.
