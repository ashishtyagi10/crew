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
    LlmPlanner, Planner, StubPlanner,
};
use crew_render::CellView;

use crate::swarm::bridge::SwarmHandle;
use crate::swarm::plan::{plan_goal, PlanHandle};
use crate::swarm::view::swarm_cells;

// Backend selection + execution helpers live in `swarm::backend`; re-exported
// so callers (and the tests) keep addressing them through this module.
pub(crate) use crate::swarm::backend::{
    backend_for, banner, executor, jobs_from_lines, running, Backend, GOAL_BUDGET_MICROS_USD,
    GOAL_FANOUT, PLAN_TIER, WORK_MAX_TOKENS,
};

/// The lifecycle of a swarm pane.
pub(crate) enum SwarmState {
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
                    model: None,
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

/// The swarm view has no cursor or input; the only key it answers is Escape →
/// close, matching the Far/Markdown/Chat panes.
pub(crate) fn esc_closes(key: &winit::keyboard::Key, pressed: bool) -> bool {
    pressed
        && matches!(
            key,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
        )
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

#[cfg(test)]
#[path = "swarmpane_tests.rs"]
mod tests;
