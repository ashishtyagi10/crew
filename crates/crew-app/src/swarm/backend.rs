//! Backend selection and execution helpers for swarm panes: which planner /
//! agent factory a `/goal` or `/batch` run uses (real LLM when a key is set,
//! offline stubs otherwise), plus the small pane-side helpers that don't
//! touch pane state. Split from `swarmpane` to keep it under the line cap.
use std::sync::Arc;

use crew_hive::agent::StubFactory;
use crew_hive::{
    AgentFactory, AnthropicProvider, ApiFactory, Budget, Fleet, Job, ModelTier, TaskGraph,
};
use crew_render::CellView;

use crate::swarm::bridge::SwarmHandle;
use crate::swarmpane::SwarmState;

/// How many parallel leaves the stub planner decomposes a goal into.
pub(crate) const GOAL_FANOUT: usize = 3;
/// Model tier the LLM planner uses to decompose a goal (better structure).
/// Worker agents instead run at whatever per-task tier the planner assigns.
pub(crate) const PLAN_TIER: ModelTier = ModelTier::Standard;
/// Per-task output token cap for worker agents.
pub(crate) const WORK_MAX_TOKENS: u32 = 2048;
/// Model tier for `/batch` jobs (no planner assigns one; keep it cost-conscious).
pub(crate) const BATCH_TIER: ModelTier = ModelTier::Cheap;

/// Which planning + execution backend a goal pane uses.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Backend {
    /// Real LLM planner + API worker agents (an API key is present).
    Llm,
    /// Deterministic stub planner + stub agents (offline fallback).
    Stub,
}

/// Pick the backend from whether an API key is available. Pure + testable; the
/// side-effecting `from_env` lookup happens once in [`SwarmPane::for_goal`].
pub(crate) fn backend_for(has_api_key: bool) -> Backend {
    if has_api_key {
        Backend::Llm
    } else {
        Backend::Stub
    }
}

/// Build a `Running` state: spawn the engine for `graph` with `factory`, capped
/// by `budget` when set.
pub(crate) fn running(
    graph: TaskGraph,
    factory: Arc<dyn AgentFactory>,
    budget: Option<Budget>,
) -> SwarmState {
    SwarmState::Running {
        handle: SwarmHandle::spawn(graph, factory, 4, budget),
        fleet: Fleet::new(),
    }
}

/// Choose the execution backend for a graph that's already planned: real,
/// budget-capped API agents when `ANTHROPIC_API_KEY` is set, else free stub
/// agents. (`/goal` selects its own backend because it also needs a planner.)
pub(crate) fn executor() -> (Arc<dyn AgentFactory>, Option<Budget>) {
    let provider = AnthropicProvider::from_env().ok();
    match backend_for(provider.is_some()) {
        Backend::Llm => {
            let provider = provider.expect("Llm backend implies a provider");
            let factory = Arc::new(ApiFactory::new(Arc::new(provider), WORK_MAX_TOKENS));
            let budget = Some(Budget {
                max_micros_usd: crew_hive::Budget::DEFAULT_MICROS_USD,
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

/// Lay `text` across row 0 as a single line of cell views (truncated to
/// `cols`). Via a ratatui buffer so wide characters in a goal keep alignment.
pub(crate) fn banner(text: &str, cols: u16) -> Vec<CellView> {
    if cols == 0 {
        return vec![];
    }
    let mut buf = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, cols, 1));
    buf.set_line(0, 0, &ratatui::text::Line::raw(text), cols);
    crate::tui::to_cells(&buf)
}
