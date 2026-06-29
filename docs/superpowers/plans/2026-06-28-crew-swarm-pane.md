# /crew → Swarm Pane (reuse-based) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.
>
> **Supersedes** `docs/superpowers/plans/2026-06-27-crew-app-swarm-integration.md` (its Tasks 1–2 are already done on this branch: `swarm/bridge.rs`, `swarm/view.rs`). This plan reuses that code and finishes the user-facing pane wired to `/crew`, plus retires the broker.

**Goal:** Repoint `/crew` from the multi-agent broker to a swarm pane: type a goal, the hive plans it into a task DAG and runs a bounded agent pool on a worker thread, rendered live as the constellation/heatmap glyph grid with a bottom goal bar. Retire the broker.

**Architecture:** Reuse the existing, tested `swarm::bridge::SwarmHandle` (off-thread scheduler → mpsc `HiveEvent` drain) and `swarm::view::swarm_cells` (Fleet → CellViews + HUD). Add: (1) `crew_hive::ApiFactory` (native-API agents), (2) `swarm::plan` (goal → `TaskGraph` on a short worker thread, non-blocking), (3) `swarm::pane::SwarmPane` (goal bar + state machine Idle→Planning→Running, draining the engine each frame), wired to `/crew`. With no `ANTHROPIC_API_KEY`, `/crew` runs a keyless `StubPlanner`+`StubFactory` demo (the whole pane animates with zero external calls); with a key it uses `LlmPlanner`+`ApiFactory`.

**Tech Stack:** Rust 2021, `crew-hive` (already a crew-app dep), `tokio` (current-thread runtime on worker threads only — never on the UI thread), `std::sync::mpsc`, existing `crew-render` `CellView` path. No new external dependencies.

## Global Constraints

- **Hard 200-line maximum per `.rs` file**, total. crew-app files are tight — split into submodules.
- **No new external dependencies.** `crew-hive` + `tokio` are already in `crates/crew-app/Cargo.toml`.
- **Do not modify the already-tested `swarm/bridge.rs` or `swarm/view.rs`** unless a task explicitly says so. Reuse them as-is.
- **No tokio runtime on the UI thread.** Background work uses `std::thread` + a worker-local current-thread tokio runtime + `std::sync::mpsc` drained each frame (mirror `swarm/bridge.rs`).
- **No overlay UI.** In-pane UI renders to `Vec<CellView>`; panes are tiles in the auto-tiling grid; panels are fieldset cards.
- **Pass-through keys.** Only act on keys the focused pane is handed by `keys.rs`.
- **Warning-free.** `cargo clippy --workspace --all-targets` zero warnings; remove dead code rather than `#[allow]`.
- **Bring-your-own-provider.** Default model tier `Standard` (`claude-sonnet-4-6`). With no key, fall back to the stub demo — never crash.
- **Commit after each task**, ending messages with: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

## File Structure

**Reused unchanged:** `crates/crew-app/src/swarm/bridge.rs`, `crates/crew-app/src/swarm/view.rs`.

**Created:**
- `crates/crew-hive/src/apiagent/factory.rs` — `ApiFactory` (or inline in `apiagent/mod.rs` if it stays ≤ 200).
- `crates/crew-app/src/swarm/plan.rs` — `plan_goal` / `PlanHandle` (goal → `TaskGraph` off-thread).
- `crates/crew-app/src/swarm/pane.rs` — `SwarmPane` struct, `new`, `poll`, `cells`.
- `crates/crew-app/src/swarm/panestate.rs` — `SwarmStatus` enum + the factory/planner builder.
- `crates/crew-app/src/swarm/panekeys.rs` — `SwarmPane::on_key`.
- `crates/crew-app/src/swarm/panerender.rs` — `SwarmPane::cells` body (goal bar + `swarm_cells`).

**Modified:**
- `crates/crew-hive/src/lib.rs` — export `ApiFactory`.
- `crates/crew-app/src/swarm/mod.rs` — add `plan`, `pane`, `panestate`, `panekeys`, `panerender` modules; re-export `SwarmPane`.
- `crates/crew-app/src/swarm/tests.rs` — tests for `plan` and `SwarmPane` (keep additive).
- `crates/crew-app/src/pane.rs` — `PaneContent::Swarm(Box<SwarmPane>)` + `cells()` + `title_text()`.
- `crates/crew-app/src/keys.rs` — route keys to `PaneContent::Swarm`.
- `crates/crew-app/src/poll.rs` — poll `PaneContent::Swarm`.
- `crates/crew-app/src/chatspawn.rs` — rewrite `spawn_crew_pane` (no subprocess); delete `crew_broker_cmd`.
- `crates/crew-app/src/main.rs` — remove the `--broker-plugin` branch.

**Deleted (broker retirement):** `crates/crew-plugin/src/broker/`, `crates/crew-plugin/src/bin/crew-broker-plugin.rs`, the broker re-exports in `crates/crew-plugin/src/lib.rs`.

---

## Task 1: crew-hive — `ApiFactory`

**Files:**
- Modify: `crates/crew-hive/src/apiagent/mod.rs`
- Modify: `crates/crew-hive/src/lib.rs` (export)
- Test: `crates/crew-hive/src/apiagent/tests.rs`

**Interfaces:**
- Produces: `ApiFactory::new(provider: Arc<dyn Provider>, tier: ModelTier, max_tokens: u32) -> ApiFactory` implementing `AgentFactory` (every `make()` → an `ApiAgent`). Re-exported as `crew_hive::ApiFactory`.

- [ ] **Step 1: Write the failing test** — append to `crates/crew-hive/src/apiagent/tests.rs`:

```rust
#[test]
fn api_factory_makes_an_agent() {
    use crate::agent::AgentFactory;
    use crate::graph::{AgentKind, ModelTier};
    use crate::provider::MockProvider;
    use std::sync::Arc;

    let provider = Arc::new(MockProvider::new("ok"));
    let factory = crate::apiagent::ApiFactory::new(provider, ModelTier::Standard, 256);
    let _agent = factory.make(&AgentKind::Api { system: None });
}
```

If `MockProvider::new` differs, check `crates/crew-hive/src/provider/mock.rs` and match its constructor.

- [ ] **Step 2: Run test, verify it fails** — `cargo test -p crew-hive api_factory_makes_an_agent` → FAIL (`ApiFactory` not found).

- [ ] **Step 3: Implement `ApiFactory`** — append to `crates/crew-hive/src/apiagent/mod.rs` (it already imports `Agent`, `AgentKind`, `ModelTier`, `Provider`, `Arc`):

```rust
// ---------------------------------------------------------------------------
// ApiFactory
// ---------------------------------------------------------------------------

use crate::agent::AgentFactory;

/// Agent factory making native [`ApiAgent`]s that share one provider. `make`
/// ignores per-task tier (the scheduler passes only `AgentKind`); all agents
/// use the configured `tier`. Per-task tiers are a follow-up.
pub struct ApiFactory {
    provider: Arc<dyn Provider>,
    tier: ModelTier,
    max_tokens: u32,
}

impl ApiFactory {
    pub fn new(provider: Arc<dyn Provider>, tier: ModelTier, max_tokens: u32) -> Self {
        Self {
            provider,
            tier,
            max_tokens,
        }
    }
}

impl AgentFactory for ApiFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        Box::new(ApiAgent::new(
            Arc::clone(&self.provider),
            self.tier,
            self.max_tokens,
        ))
    }
}
```

- [ ] **Step 4: Export it** — in `crates/crew-hive/src/lib.rs`, change the ApiAgent re-export to:

```rust
pub use apiagent::{ApiAgent, ApiFactory};
```

- [ ] **Step 5: Run test, verify it passes** — `cargo test -p crew-hive api_factory_makes_an_agent` → PASS.

- [ ] **Step 6: Size + clippy** — `wc -l crates/crew-hive/src/apiagent/mod.rs` ≤ 200 (if over, move `ApiFactory` to `crates/crew-hive/src/apiagent/factory.rs` as `mod factory; pub use factory::ApiFactory;`). `cargo clippy -p crew-hive --all-targets` → zero warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-hive/
git commit -m "feat(hive): ApiFactory — native-API agent factory

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: `swarm::plan` — goal → TaskGraph off-thread

**Files:**
- Create: `crates/crew-app/src/swarm/plan.rs`
- Modify: `crates/crew-app/src/swarm/mod.rs` (`pub mod plan;`)
- Test: `crates/crew-app/src/swarm/tests.rs`

**Interfaces:**
- Consumes: `crew_hive::{Planner, TaskGraph}`.
- Produces:
  - `struct PlanHandle` with `fn try_take(&self) -> Option<Result<TaskGraph, String>>` (non-blocking; `None` until the planner thread finishes).
  - `fn plan_goal(goal: String, planner: Arc<dyn Planner>) -> PlanHandle` — spawns a worker thread with a current-thread tokio runtime that runs `planner.plan(&goal)` and sends the result back.

- [ ] **Step 1: Write the failing test** — append to `crates/crew-app/src/swarm/tests.rs`:

```rust
#[test]
fn plan_goal_produces_a_graph() {
    use crate::swarm::plan::plan_goal;
    use crew_hive::{Planner, StubPlanner};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let planner: Arc<dyn Planner> = Arc::new(StubPlanner { fanout: 3 });
    let handle = plan_goal("build a thing".into(), planner);

    let start = Instant::now();
    let result = loop {
        if let Some(r) = handle.try_take() {
            break r;
        }
        assert!(start.elapsed() < Duration::from_secs(5), "planner timed out");
        std::thread::yield_now();
    };
    let graph = result.expect("stub planner should succeed");
    // StubPlanner { fanout: 3 } makes 3 leaves + 1 merge = 4 tasks.
    assert_eq!(graph.len(), 4);
}
```

- [ ] **Step 2: Run test, verify it fails** — `cargo test -p crew-app plan_goal_produces_a_graph` → FAIL (module `plan` not found).

- [ ] **Step 3: Implement `plan.rs`** — create `crates/crew-app/src/swarm/plan.rs`:

```rust
//! Goal → `TaskGraph` on a short-lived worker thread. Planning is an async LLM
//! call; running it off the UI thread keeps the frame loop non-blocking. The
//! result is delivered over a `std::sync::mpsc` channel, drained each frame.
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

use crew_hive::{Planner, TaskGraph};

/// Handle to an in-flight plan. `try_take` returns `None` until the planner
/// thread finishes, then `Some(Ok(graph))` or `Some(Err(message))` once.
pub struct PlanHandle {
    rx: Receiver<Result<TaskGraph, String>>,
}

impl PlanHandle {
    /// Non-blocking check for the planned graph.
    pub fn try_take(&self) -> Option<Result<TaskGraph, String>> {
        self.rx.try_recv().ok()
    }
}

/// Spawn a worker thread that plans `goal` into a `TaskGraph` and sends the
/// result back. The thread owns a current-thread tokio runtime.
pub fn plan_goal(goal: String, planner: Arc<dyn Planner>) -> PlanHandle {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(Err(format!("runtime: {e}")));
                return;
            }
        };
        let result = rt.block_on(async move { planner.plan(&goal).await.map_err(|e| e.to_string()) });
        let _ = tx.send(result);
    });
    PlanHandle { rx }
}
```

- [ ] **Step 4: Register module** — in `crates/crew-app/src/swarm/mod.rs`, add `pub mod plan;` (keep existing `pub mod bridge; pub mod view;` and the `#[cfg(test)] mod tests;`).

- [ ] **Step 5: Run test, verify it passes** — `cargo test -p crew-app plan_goal_produces_a_graph` → PASS.

- [ ] **Step 6: Size + clippy** — `wc -l crates/crew-app/src/swarm/plan.rs` ≤ 200. `cargo clippy -p crew-app --all-targets` → zero warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-app/src/swarm/plan.rs crates/crew-app/src/swarm/mod.rs crates/crew-app/src/swarm/tests.rs
git commit -m "feat(swarm): plan_goal — goal -> TaskGraph off the UI thread

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `SwarmPane` core (state, keys, render, poll)

**Files:**
- Create: `crates/crew-app/src/swarm/panestate.rs`
- Create: `crates/crew-app/src/swarm/pane.rs`
- Create: `crates/crew-app/src/swarm/panekeys.rs`
- Create: `crates/crew-app/src/swarm/panerender.rs`
- Modify: `crates/crew-app/src/swarm/mod.rs` (modules + `pub use pane::SwarmPane;`)
- Test: `crates/crew-app/src/swarm/tests.rs`

**Interfaces:**
- Consumes: `crate::swarm::{bridge::SwarmHandle, view::swarm_cells, plan::{plan_goal, PlanHandle}}`, `crew_hive::{Fleet, Planner, AgentFactory, StubPlanner, StubFactory, LlmPlanner, ApiFactory, AnthropicProvider, Provider, ModelTier}`, `crew_render::CellView`, `winit::event::KeyEvent`.
- Produces:
  - `enum SwarmStatus { Idle, Planning, Running, Failed(String) }`
  - `struct SwarmPane { pub goal: String, pub status: SwarmStatus, pub plan: Option<PlanHandle>, pub engine: Option<SwarmHandle>, pub fleet: Fleet }`
  - `SwarmPane::new() -> SwarmPane`, `Default`
  - `SwarmPane::poll(&mut self) -> bool`
  - `SwarmPane::cells(&self, cols: u16, rows: u16) -> Vec<CellView>`
  - `SwarmPane::on_key(&mut self, key: &KeyEvent)`
  - `fn build_planner_and_factory() -> (Arc<dyn Planner>, Arc<dyn AgentFactory>)` in `panestate.rs` (LLM+Api when `ANTHROPIC_API_KEY` set, else stub demo).

**Decisions (resolved by controller):**
- Keep the existing `swarm_cells` HUD (row 0); render the swarm into `rows-1` so the bottom row hosts the goal bar. (Reuses tested `view.rs` unchanged.)
- Concurrency `8`, stub `fanout` `6`, `max_tokens` `2048`, tier `Standard`.
- No `Done` status: completion is visible via the HUD's `done/failed` counters and the engine channel closing. Keep the enum minimal.

- [ ] **Step 1: Write the failing tests** — append to `crates/crew-app/src/swarm/tests.rs`:

```rust
mod swarmpane {
    use crate::swarm::pane::SwarmPane;
    use crate::swarm::panestate::SwarmStatus;
    use winit::event::{ElementState, KeyEvent};
    use winit::keyboard::{Key, NamedKey};

    fn press(key: Key) -> KeyEvent {
        KeyEvent {
            physical_key: winit::keyboard::PhysicalKey::Unidentified(
                winit::keyboard::NativeKeyCode::Unidentified,
            ),
            logical_key: key,
            text: None,
            location: winit::keyboard::KeyLocation::Standard,
            state: ElementState::Pressed,
            repeat: false,
            platform_specific: Default::default(),
        }
    }

    #[test]
    fn idle_pane_renders_hint_and_goal_bar() {
        let pane = SwarmPane::new();
        let text: String = pane.cells(40, 10).iter().map(|c| c.c).collect();
        assert!(text.to_lowercase().contains("goal"), "missing hint/bar: {text:?}");
    }

    #[test]
    fn typing_edits_goal_and_backspace_deletes() {
        let mut pane = SwarmPane::new();
        pane.on_key(&press(Key::Character("h".into())));
        pane.on_key(&press(Key::Character("i".into())));
        assert_eq!(pane.goal, "hi");
        pane.on_key(&press(Key::Named(NamedKey::Backspace)));
        assert_eq!(pane.goal, "h");
    }

    #[test]
    fn enter_on_empty_goal_stays_idle() {
        let mut pane = SwarmPane::new();
        pane.on_key(&press(Key::Named(NamedKey::Enter)));
        assert!(matches!(pane.status, SwarmStatus::Idle));
        assert!(pane.plan.is_none() && pane.engine.is_none());
    }

    #[test]
    fn enter_with_goal_begins_planning() {
        let mut pane = SwarmPane::new();
        pane.goal = "build a thing".into();
        pane.on_key(&press(Key::Named(NamedKey::Enter)));
        assert!(matches!(pane.status, SwarmStatus::Planning));
        assert!(pane.plan.is_some());
    }
}
```

If direct `KeyEvent` construction fails (winit `#[non_exhaustive]`), refactor `on_key` to delegate to a pure `fn apply_key(&mut self, logical: &Key, pressed: bool)` and test that instead. Decide when you see Step 2's compiler output.

- [ ] **Step 2: Run tests, verify they fail** — `cargo test -p crew-app swarmpane` → FAIL (modules not found). Apply the `KeyEvent` fallback now if needed.

- [ ] **Step 3: Implement `panestate.rs`** — create `crates/crew-app/src/swarm/panestate.rs`:

```rust
//! `SwarmPane` lifecycle state + the planner/factory builder.
use std::sync::Arc;

use crew_hive::{
    AgentFactory, AnthropicProvider, ApiFactory, LlmPlanner, ModelTier, Planner, Provider,
    StubFactory, StubPlanner,
};

/// Stub fan-out for the keyless demo.
pub const STUB_FANOUT: usize = 6;
const MAX_TOKENS: u32 = 2048;

/// Where a `/crew` swarm pane is in its lifecycle.
#[derive(Debug, Clone, PartialEq)]
pub enum SwarmStatus {
    /// Accepting goal input.
    Idle,
    /// Planning the goal into a task graph (off-thread).
    Planning,
    /// The swarm is running.
    Running,
    /// Planning/setup failed; carries the message to show.
    Failed(String),
}

/// Build a planner + agent factory. With `ANTHROPIC_API_KEY` set, use the LLM
/// planner + native-API agents; otherwise fall back to a keyless stub demo so
/// the pane always animates without crashing.
pub fn build_planner_and_factory() -> (Arc<dyn Planner>, Arc<dyn AgentFactory>) {
    match (AnthropicProvider::from_env(), AnthropicProvider::from_env()) {
        (Ok(plan_p), Ok(agent_p)) => {
            let planner: Arc<dyn Planner> = Arc::new(LlmPlanner {
                provider: plan_p,
                tier: ModelTier::Standard,
            });
            let agent_provider: Arc<dyn Provider> = Arc::new(agent_p);
            let factory: Arc<dyn AgentFactory> =
                Arc::new(ApiFactory::new(agent_provider, ModelTier::Standard, MAX_TOKENS));
            (planner, factory)
        }
        _ => (
            Arc::new(StubPlanner { fanout: STUB_FANOUT }),
            Arc::new(StubFactory),
        ),
    }
}
```

- [ ] **Step 4: Implement `panerender.rs`** — create `crates/crew-app/src/swarm/panerender.rs`:

```rust
//! Render a `SwarmPane`: the swarm grid (with HUD) fills the top; a `> goal`
//! bar occupies the bottom row.
use crew_render::CellView;

use super::pane::SwarmPane;
use super::panestate::SwarmStatus;
use super::view::swarm_cells;

const BG: (u8, u8, u8) = (0, 0, 0);
const FG: (u8, u8, u8) = (210, 210, 220);

fn put_str(out: &mut Vec<CellView>, row: u16, text: &str, cols: u16, fg: (u8, u8, u8)) {
    for (i, ch) in text.chars().enumerate() {
        if i as u16 >= cols {
            break;
        }
        out.push(CellView {
            col: i as u16,
            row,
            c: ch,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
}

/// Build the cell list for a `cols`×`rows` pane.
pub fn cells(pane: &SwarmPane, cols: u16, rows: u16) -> Vec<CellView> {
    let mut out = Vec::new();
    if cols == 0 || rows == 0 {
        return out;
    }
    let goal_row = rows - 1;
    let swarm_rows = rows - 1; // reserve the bottom row for the goal bar

    match &pane.engine {
        Some(engine) => {
            // Swarm grid + HUD (swarm_cells reserves its own row 0 for the HUD).
            out.extend(swarm_cells(engine.graph(), &pane.fleet, cols, swarm_rows));
        }
        None => {
            let hint = match &pane.status {
                SwarmStatus::Planning => "Planning…".to_string(),
                SwarmStatus::Failed(e) => format!("error: {e}"),
                _ => "Enter a goal for the swarm…".to_string(),
            };
            put_str(&mut out, 0, &hint, cols, FG);
        }
    }

    put_str(&mut out, goal_row, &format!("> {}", pane.goal), cols, FG);
    out
}
```

- [ ] **Step 5: Implement `panekeys.rs`** — create `crates/crew-app/src/swarm/panekeys.rs`:

```rust
//! Key handling for a swarm pane: edit the goal, Enter launches planning,
//! Esc cancels a running swarm.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::pane::SwarmPane;
use super::panestate::SwarmStatus;
use super::plan::plan_goal;
use super::panestate::build_planner_and_factory;

impl SwarmPane {
    /// Handle one winit key event (presses only).
    pub fn on_key(&mut self, key: &KeyEvent) {
        if !key.state.is_pressed() {
            return;
        }
        match &key.logical_key {
            Key::Named(NamedKey::Enter) => self.launch(),
            Key::Named(NamedKey::Escape) => self.cancel(),
            Key::Named(NamedKey::Backspace) => {
                self.goal.pop();
            }
            Key::Named(NamedKey::Space) => self.goal.push(' '),
            Key::Character(s) => {
                if let Some(c) = s.chars().next() {
                    self.goal.push(c);
                }
            }
            _ => {}
        }
    }

    /// Begin planning the current goal (no-op if empty or already busy).
    fn launch(&mut self) {
        if self.goal.trim().is_empty()
            || matches!(self.status, SwarmStatus::Planning | SwarmStatus::Running)
        {
            return;
        }
        self.fleet = crew_hive::Fleet::new();
        self.engine = None;
        let (planner, _factory) = build_planner_and_factory();
        // Keep the factory for when planning completes (poll() spawns the engine).
        self.pending_factory = Some(_factory);
        self.plan = Some(plan_goal(self.goal.clone(), planner));
        self.status = SwarmStatus::Planning;
    }

    /// Signal cooperative cancellation to a running swarm.
    fn cancel(&mut self) {
        if let Some(engine) = &self.engine {
            engine.cancel();
        }
    }
}
```

Note: `launch` needs to stash the factory until planning finishes. Add a `pending_factory: Option<Arc<dyn AgentFactory>>` field to `SwarmPane` in Step 6.

- [ ] **Step 6: Implement `pane.rs`** — create `crates/crew-app/src/swarm/pane.rs`:

```rust
//! The `/crew` swarm pane: a goal typed on the bottom bar is planned off-thread
//! (`super::plan`) then executed by the engine bridge (`super::bridge`); the
//! live `Fleet` renders as a glyph grid each frame (`super::panerender`).
use std::sync::Arc;

use crew_hive::{AgentFactory, Fleet};
use crew_render::CellView;

use super::bridge::SwarmHandle;
use super::panestate::SwarmStatus;
use super::plan::PlanHandle;

const CONCURRENCY: usize = 8;

pub struct SwarmPane {
    pub goal: String,
    pub status: SwarmStatus,
    pub plan: Option<PlanHandle>,
    pub engine: Option<SwarmHandle>,
    pub fleet: Fleet,
    pub pending_factory: Option<Arc<dyn AgentFactory>>,
}

impl SwarmPane {
    pub fn new() -> Self {
        Self {
            goal: String::new(),
            status: SwarmStatus::Idle,
            plan: None,
            engine: None,
            fleet: Fleet::new(),
            pending_factory: None,
        }
    }

    /// Per-frame work: pick up a finished plan and spawn the engine; drain
    /// engine events into the fleet. Returns true if anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;

        // 1. Planning → Running transition.
        if let Some(plan) = &self.plan {
            if let Some(result) = plan.try_take() {
                self.plan = None;
                changed = true;
                match result {
                    Ok(graph) => {
                        if let Some(factory) = self.pending_factory.take() {
                            self.engine =
                                Some(SwarmHandle::spawn(graph, factory, CONCURRENCY));
                            self.status = SwarmStatus::Running;
                        }
                    }
                    Err(e) => self.status = SwarmStatus::Failed(e),
                }
            }
        }

        // 2. Drain engine events.
        if let Some(engine) = &self.engine {
            let before = self.fleet.len();
            engine.drain(&mut self.fleet);
            if self.fleet.len() != before {
                changed = true;
            }
            // Token/cost updates also change cells; redraw while live.
            if self.fleet.totals().live > 0 {
                changed = true;
            }
        }
        changed
    }

    /// Render to GPU cells.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        super::panerender::cells(self, cols, rows)
    }
}

impl Default for SwarmPane {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 7: Register modules** — in `crates/crew-app/src/swarm/mod.rs`:

```rust
//! Swarm integration: off-thread scheduler bridge + Fleet→CellViews renderer
//! + the `/crew` swarm pane.
pub mod bridge;
pub mod pane;
pub mod panekeys;
pub mod panerender;
pub mod panestate;
pub mod plan;
#[cfg(test)]
mod tests;
pub mod view;

pub use pane::SwarmPane;
```

- [ ] **Step 8: Run tests, verify they pass** — `cargo test -p crew-app swarmpane` → PASS (or the adjusted set per the Step 1 fallback).

- [ ] **Step 9: Size + clippy** — `wc -l crates/crew-app/src/swarm/*.rs` (every file ≤ 200). `cargo clippy -p crew-app --all-targets` → zero warnings.

- [ ] **Step 10: Commit**

```bash
git add crates/crew-app/src/swarm/
git commit -m "feat(swarm): SwarmPane — goal bar + Planning→Running state machine

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Wire `/crew` to the swarm pane

**Files:**
- Modify: `crates/crew-app/src/pane.rs`
- Modify: `crates/crew-app/src/keys.rs`
- Modify: `crates/crew-app/src/poll.rs`
- Modify: `crates/crew-app/src/chatspawn.rs`
- Test: `crates/crew-app/src/pane.rs` (tests module)

**Interfaces:**
- Consumes: `crate::swarm::SwarmPane`.
- Produces: `/crew` opens a focused `SwarmPane`; `title_text()` returns `"crew"`.

- [ ] **Step 1: Write the failing test** — add to the `#[cfg(test)] mod tests` in `crates/crew-app/src/pane.rs`:

```rust
#[test]
fn swarm_pane_title_is_crew() {
    let p = Pane {
        content: PaneContent::Swarm(Box::new(crate::swarm::SwarmPane::new())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        activity: false,
        bell: false,
    };
    assert_eq!(p.title_text(), "crew");
}
```

- [ ] **Step 2: Run test, verify it fails** — `cargo test -p crew-app swarm_pane_title_is_crew` → FAIL (no `Swarm` variant).

- [ ] **Step 3: Add the `Swarm` variant + arms in `pane.rs`** — add `use crate::swarm::SwarmPane;`; add the variant:

```rust
pub enum PaneContent {
    Terminal(Box<TermPane>),
    Chat(ChatPane),
    Settings(SettingsPane),
    Far(FarPane),
    Swarm(Box<SwarmPane>),
}
```

Add the `title_text()` arm: `PaneContent::Swarm(_) => "crew".into(),`
Add the `cells()` arm: `PaneContent::Swarm(s) => s.cells(self.grid.cols, self.grid.rows),`

- [ ] **Step 4: Route keys in `keys.rs`** — next to `PaneContent::Chat(c) => c.on_key(event),` add:

```rust
                PaneContent::Swarm(s) => s.on_key(event),
```

- [ ] **Step 5: Poll in `poll.rs`** — change the `Settings | Far` arm so Swarm is polled:

```rust
                PaneContent::Swarm(s) => s.poll(),
                PaneContent::Settings(_) | PaneContent::Far(_) => false,
```

- [ ] **Step 6: Rewrite `spawn_crew_pane` (no subprocess)** — in `crates/crew-app/src/chatspawn.rs`, replace `spawn_crew_pane`:

```rust
    /// Spawn the `/crew` pane: an in-process swarm view. Typing a goal on the
    /// bottom bar plans + runs a `crew-hive` swarm (see `crate::swarm`); the
    /// live fleet renders as a glyph grid. Named "crew" for its title bar.
    pub(crate) fn spawn_crew_pane(&mut self) {
        use crate::pane::{Pane, PaneContent};
        use crate::swarm::SwarmPane;
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            content: PaneContent::Swarm(Box::new(SwarmPane::new())),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: Some("crew".to_string()),
            activity: false,
            bell: false,
        });
        self.focus_new_pane();
    }
```

Then delete the now-unused `crew_broker_cmd` function. Leave `spawn_chat_pane`, `spawn_plugin_pane`, `echo_plugin_cmd`, `orchestrator_plugin_cmd` intact (used by `chords.rs`). Trim any imports left unused.

- [ ] **Step 7: Run test + build** — `cargo test -p crew-app swarm_pane_title_is_crew` → PASS. `cargo check --workspace` → clean (broker still present; removed next task).

- [ ] **Step 8: Clippy** — `cargo clippy -p crew-app --all-targets` → zero warnings. (`Swarm` is boxed, so `large_enum_variant` should not fire.)

- [ ] **Step 9: Commit**

```bash
git add crates/crew-app/src/pane.rs crates/crew-app/src/keys.rs crates/crew-app/src/poll.rs crates/crew-app/src/chatspawn.rs
git commit -m "feat(app): /crew opens the in-process swarm pane

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Retire the broker

**Files:**
- Delete: `crates/crew-plugin/src/broker/`, `crates/crew-plugin/src/bin/crew-broker-plugin.rs`
- Modify: `crates/crew-plugin/src/lib.rs`, `crates/crew-app/src/main.rs`

**Interfaces:** Produces a workspace with no broker code/flag/binary. Echo + orchestrator chat plugins and `ChatPane` remain functional.

- [ ] **Step 1: Delete broker source + binary**

```bash
git rm -r crates/crew-plugin/src/broker
git rm crates/crew-plugin/src/bin/crew-broker-plugin.rs
```

- [ ] **Step 2: Remove broker from `crew-plugin/src/lib.rs`** — delete `mod broker;` and the entire `pub use broker::{ … };` block. Result:

```rust
mod echo;
mod host;
mod orchestrator;
mod protocol;
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{PluginCommand, PluginEvent};
```

- [ ] **Step 3: Remove the `--broker-plugin` branch from `main.rs`** — delete the comment block and:

```rust
    if std::env::args().skip(1).any(|a| a == "--broker-plugin") {
        return crew_plugin::run_broker_stdio();
    }
```

- [ ] **Step 4: Verify no dangling references** — `grep -rn "broker\|run_broker_stdio\|known_adapters\|parse_routing\|crew-broker-plugin\|--broker-plugin" crates/ --include='*.rs' --include='*.toml'` → no matches (remove stragglers, e.g. stale comments in `chatspawn.rs`).

- [ ] **Step 5: Full gate** — `cargo fmt --all`; `cargo check --workspace` clean; `cargo clippy --workspace --all-targets` zero warnings; `cargo test --workspace` all green (broker tests gone; echo/orchestrator/chat/swarm tests pass).

- [ ] **Step 6: 200-line cap** — `find crates -name '*.rs' -exec wc -l {} + | awk '$1 > 200 {print}'` → no output.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: retire the multi-agent broker (superseded by the swarm)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Documentation

**Files:** `README.md`, `docs/CREW.md`

- [ ] **Step 1: Update `/crew` docs** — in `README.md` and `docs/CREW.md`, replace any broker / `--broker-plugin` / claude⇆codex⇆opencode description with: `/crew` opens an in-process **swarm pane** — type a goal, the hive plans a task DAG and runs a bounded agent pool, rendered live as a constellation/heatmap glyph grid with a live HUD and a bottom goal bar. Keys: type to edit the goal, **Enter** to launch, **Esc** to cancel. With `ANTHROPIC_API_KEY` set it runs live (LLM planner + native-API agents); without a key it runs a stub demo. Match existing style; add, don't rewrite surrounding content.

- [ ] **Step 2: fmt + check** — `cargo fmt --all && cargo check --workspace` → clean.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/CREW.md
git commit -m "docs: /crew now opens the in-process swarm pane

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Final Verification (gate)

- [ ] `cargo fmt --all -- --check` — clean
- [ ] `cargo clippy --workspace --all-targets` — zero warnings
- [ ] `cargo test --workspace` — all green
- [ ] `find crates -name '*.rs' -exec wc -l {} + | awk '$1 > 200 {print}'` — no output
- [ ] `grep -rn "broker" crates/ --include='*.rs'` — no matches
- [ ] Reconcile with `main` (4 docs/version commits ahead): merge `main` into the branch (or rebase), resolving README/CREW.md/lib.rs doc overlaps, before merging back.
- [ ] Manual smoke (optional): `cargo run -p crew-app`, type `/crew`, enter a goal — keyless stub demo animates the constellation; with `ANTHROPIC_API_KEY`, live agents run. Esc cancels.

---

## Self-Review Notes

- **Reuse honored:** `swarm/bridge.rs` and `swarm/view.rs` are untouched; Tasks 2–3 consume them. Only `swarm/mod.rs` and `swarm/tests.rs` (additive) change.
- **Spec coverage:** swarm-view-first + goal bar (Task 3 render), in-process worker threads / no UI-thread tokio (Tasks 2–3 reuse bridge), native `ApiAgent` backend (Task 1 + Task 3 builder), keyless stub demo (Task 3 builder — also unblocks GUI verification without a key), Esc cancel (Task 3 → bridge `cancel`), broker retired incl. flag/binary/re-exports (Task 5), `/crew` repointed (Task 4), docs (Task 6).
- **Type consistency:** `SwarmPane` fields (`goal`/`status`/`plan`/`engine`/`fleet`/`pending_factory`) match across `pane.rs`/`panekeys.rs`/`panerender.rs`/`panestate.rs`/tests; `SwarmStatus` variants (`Idle`/`Planning`/`Running`/`Failed`) consistent; `PlanHandle::try_take`, `SwarmHandle::{spawn,drain,cancel,graph}`, `swarm_cells(graph,fleet,cols,rows)` used per their real signatures.
- **Known winit risk:** direct `KeyEvent` construction in tests may fail if `#[non_exhaustive]`; Task 3 Step 1 carries the `apply_key` fallback.
- **Branch is 4 docs/version commits behind main** — reconciled at the final gate, not mid-flight.
