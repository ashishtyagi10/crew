//! A live swarm pane. Two entry points:
//!
//! - `/goal <text>` → [`SwarmPane::for_goal`] first plans the goal into a graph
//!   off the UI thread (via [`plan_goal`]), shows a "planning…" banner, then runs
//!   the resulting graph and visualises it.
//! - `/batch <file>` → [`SwarmPane::for_batch`] runs a flat list of jobs as one
//!   all-parallel graph, no planning step.
//!
//! `/goal` adapts to its environment: when `ANTHROPIC_API_KEY` is set it plans
//! with [`LlmPlanner`] and executes with real [`ApiFactory`] agents; otherwise
//! it falls back to a deterministic [`StubPlanner`] + always-succeeding stub
//! agents so the whole goal → plan → schedule → bridge → view pipeline still
//! runs live, offline, and deterministically.
use std::sync::Arc;

use crew_hive::agent::StubFactory;
use crew_hive::{
    batch_graph, AgentFactory, AnthropicProvider, ApiFactory, Budget, Fleet, GraphError, Job,
    LlmPlanner, ModelTier, Planner, StubPlanner, TaskGraph,
};
use crew_render::CellView;

use crate::swarm::bridge::SwarmHandle;
use crate::swarm::plan::{plan_goal, PlanHandle};
use crate::swarm::view::swarm_cells;

/// How many parallel leaves the stub planner decomposes a goal into.
const GOAL_FANOUT: usize = 3;
/// Model tier the LLM planner uses to decompose a goal (better structure).
/// Worker agents instead run at whatever per-task tier the planner assigns.
const PLAN_TIER: ModelTier = ModelTier::Standard;
/// Per-task output token cap for worker agents.
const WORK_MAX_TOKENS: u32 = 2048;
/// Default cost ceiling for a real-LLM `/goal` or `/batch` run ($1.00 in
/// micros-USD). The budget governor cancels the swarm once fleet spend exceeds
/// this.
const GOAL_BUDGET_MICROS_USD: u64 = 1_000_000;
/// Model tier for `/batch` jobs (no planner assigns one; keep it cost-conscious).
const BATCH_TIER: ModelTier = ModelTier::Cheap;

/// Which planning + execution backend a goal pane uses.
#[derive(Debug, PartialEq, Eq)]
enum Backend {
    /// Real LLM planner + API worker agents (an API key is present).
    Llm,
    /// Deterministic stub planner + stub agents (offline fallback).
    Stub,
}

/// Pick the backend from whether an API key is available. Pure + testable; the
/// side-effecting `from_env` lookup happens once in [`SwarmPane::for_goal`].
fn backend_for(has_api_key: bool) -> Backend {
    if has_api_key {
        Backend::Llm
    } else {
        Backend::Stub
    }
}

/// The lifecycle of a swarm pane.
enum SwarmState {
    /// Awaiting the planner thread; `goal` is echoed in the banner. `factory`
    /// is the executor chosen at goal time, used once the graph arrives, and
    /// `budget` is its optional cost ceiling.
    Planning {
        goal: String,
        plan: PlanHandle,
        factory: Arc<dyn AgentFactory>,
        budget: Option<Budget>,
    },
    /// Executing a graph; `handle` drives the engine, `fleet` accumulates events.
    Running { handle: SwarmHandle, fleet: Fleet },
    /// Planning failed; `msg` is shown in the banner.
    Failed { msg: String },
}

/// A pane that plans and/or visualises a running swarm. Cheap to drain each frame.
pub struct SwarmPane {
    state: SwarmState,
}

impl SwarmPane {
    /// Run a batch of independent `jobs` as one all-parallel swarm — no planning
    /// step, since the jobs already are the task list. Uses the real API backend
    /// (capped) when a key is set, else the offline stub backend.
    pub fn for_batch(jobs: Vec<Job>) -> Result<Self, GraphError> {
        let graph = batch_graph(jobs)?;
        let (factory, budget) = executor();
        Ok(Self {
            state: running(graph, factory, budget),
        })
    }

    /// Plan `goal` into a task graph off-thread, then run it. Uses the real LLM
    /// planner + API agents when `ANTHROPIC_API_KEY` is set, else the offline
    /// stub backend. The pane shows a planning banner until the graph is ready.
    pub fn for_goal(goal: String) -> Self {
        let provider = AnthropicProvider::from_env().ok();
        match backend_for(provider.is_some()) {
            Backend::Llm => {
                // `is_some()` was just checked, so the unwrap cannot fail.
                let provider = provider.expect("Llm backend implies a provider");
                let planner = Arc::new(LlmPlanner {
                    provider: provider.clone(),
                    tier: PLAN_TIER,
                });
                let factory = Arc::new(ApiFactory::new(Arc::new(provider), WORK_MAX_TOKENS));
                // Real API agents accrue cost — cap the run.
                let budget = Some(Budget {
                    max_micros_usd: GOAL_BUDGET_MICROS_USD,
                });
                Self::goal_with(goal, planner, factory, budget)
            }
            Backend::Stub => Self::goal_stub(goal),
        }
    }

    /// The offline path: stub planner + stub agents. Used as the no-key fallback
    /// and directly by tests for determinism. Stub agents cost nothing, so no
    /// budget is applied.
    fn goal_stub(goal: String) -> Self {
        Self::goal_with(
            goal,
            Arc::new(StubPlanner {
                fanout: GOAL_FANOUT,
            }),
            Arc::new(StubFactory),
            None,
        )
    }

    /// Start planning `goal` with `planner`, holding `factory` (and its optional
    /// `budget`) to execute the resulting graph.
    fn goal_with(
        goal: String,
        planner: Arc<dyn Planner>,
        factory: Arc<dyn AgentFactory>,
        budget: Option<Budget>,
    ) -> Self {
        Self {
            state: SwarmState::Planning {
                plan: plan_goal(goal.clone(), planner),
                goal,
                factory,
                budget,
            },
        }
    }

    /// Advance the pane one frame. Returns `true` when something changed (a plan
    /// arrived, or engine events were applied) and the pane should redraw.
    pub fn poll(&mut self) -> bool {
        match &mut self.state {
            SwarmState::Planning {
                plan,
                factory,
                budget,
                ..
            } => match plan.try_take() {
                Some(Ok(graph)) => {
                    self.state = running(graph, Arc::clone(factory), *budget);
                    true
                }
                Some(Err(e)) => {
                    self.state = SwarmState::Failed { msg: e };
                    true
                }
                None => false,
            },
            SwarmState::Running { handle, fleet } => handle.drain(fleet) > 0,
            SwarmState::Failed { .. } => false,
        }
    }

    /// Whether the pane is working — planning, or running with live tasks and not
    /// cancelled. Drives the indeterminate progress sweep on the pane border.
    pub fn is_busy(&self) -> bool {
        match &self.state {
            SwarmState::Planning { .. } => true,
            SwarmState::Running { handle, fleet } => {
                !handle.is_cancelled() && fleet.totals().live > 0
            }
            SwarmState::Failed { .. } => false,
        }
    }

    /// Render the pane for a `cols × rows` grid: a banner while planning/failed,
    /// the live constellation + HUD while running.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols == 0 || rows == 0 {
            return vec![];
        }
        match &self.state {
            SwarmState::Planning { goal, .. } => banner(&format!("planning: {goal}…"), cols),
            SwarmState::Failed { msg } => banner(&format!("plan failed: {msg}"), cols),
            SwarmState::Running { handle, fleet } => {
                let mut cells = swarm_cells(handle.graph(), fleet, cols, rows);
                if handle.is_cancelled() {
                    cells.extend(crate::swarm::view::cancelled_notice(cols, rows));
                }
                cells
            }
        }
    }
}

impl Drop for SwarmPane {
    /// Stop the background scheduler when the pane closes, so a dismissed swarm
    /// doesn't keep spawning tasks on its worker thread.
    fn drop(&mut self) {
        if let SwarmState::Running { handle, .. } = &self.state {
            handle.cancel();
        }
    }
}

/// Build a `Running` state: spawn the engine for `graph` with `factory`, capped
/// by `budget` when set.
fn running(graph: TaskGraph, factory: Arc<dyn AgentFactory>, budget: Option<Budget>) -> SwarmState {
    SwarmState::Running {
        handle: SwarmHandle::spawn(graph, factory, 4, budget),
        fleet: Fleet::new(),
    }
}

/// Choose the execution backend for a graph that's already planned: real,
/// budget-capped API agents when `ANTHROPIC_API_KEY` is set, else free stub
/// agents. (`/goal` selects its own backend because it also needs a planner.)
fn executor() -> (Arc<dyn AgentFactory>, Option<Budget>) {
    let provider = AnthropicProvider::from_env().ok();
    match backend_for(provider.is_some()) {
        Backend::Llm => {
            let provider = provider.expect("Llm backend implies a provider");
            let factory = Arc::new(ApiFactory::new(Arc::new(provider), WORK_MAX_TOKENS));
            let budget = Some(Budget {
                max_micros_usd: GOAL_BUDGET_MICROS_USD,
            });
            (factory, budget)
        }
        Backend::Stub => (Arc::new(StubFactory), None),
    }
}

/// Parse batch jobs from text: one job per non-blank line, the line serving as
/// both the (truncated) title and the prompt. Empty input yields no jobs.
pub(crate) fn jobs_from_lines(text: &str) -> Vec<Job> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| Job {
            title: l.chars().take(40).collect(),
            prompt: l.to_string(),
            tier: BATCH_TIER,
        })
        .collect()
}

/// Lay `text` across row 0 as a single line of cell views (truncated to `cols`).
fn banner(text: &str, cols: u16) -> Vec<CellView> {
    text.chars()
        .take(cols as usize)
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row: 0,
            c,
            fg: crew_theme::theme().ink,
            bg: crew_theme::theme().page_bg,
            bold: false,
            italic: false,
        })
        .collect()
}

#[cfg(test)]
#[path = "swarmpane_tests.rs"]
mod tests;
