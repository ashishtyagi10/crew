//! Swarm run configuration: provider-driven backend selection and the small
//! helpers `swarm::run_task`/`run_with` lean on. Split from `swarm.rs` to
//! keep that file inside the line cap as re-planning wired in. A child of
//! `swarm`, so `broker` items are reached through `crate::broker::`.
use std::sync::Arc;

use crew_hive::agent::StubFactory;
use crew_hive::{AgentFactory, Budget, LlmPlanner, ModelTier, Planner, StubPlanner};

use super::{Session, STUB_FANOUT, WORK_MAX_TOKENS};

/// Pick planner/factory/budget/replanner from provider discovery: real LLM
/// planning on a discovered provider; deterministic stubs when keyless. The
/// mock provider (GUI harness) plans with stubs but executes through the
/// mock, so replies stay deterministic while the full pipeline runs. The
/// REPLANNER is `Some` only on the real-provider arm — keyless and mock runs
/// keep pure cascade-cancel on failure, exactly as before.
///
/// `tools` is the session's surface (see [`Session::tools`]), attached to the
/// REAL-PROVIDER arm only. The stub factory has no model to decide with, and
/// the mock arm must stay byte-deterministic for the GUI harness — a mock
/// reply that happened to end in `@tool` would otherwise start executing shell
/// commands during a screenshot test. `None` here leaves the swarm exactly as
/// it was before tools existed.
pub(super) type Backend = (
    Arc<dyn Planner>,
    Arc<dyn AgentFactory>,
    Option<Budget>,
    String,
    Option<Arc<dyn Planner>>,
);

pub(super) fn backend(tools: Option<Arc<dyn crew_hive::tools::Tools>>) -> Backend {
    match crate::broker::discover::provider_and_model() {
        None => (
            Arc::new(StubPlanner {
                fanout: STUB_FANOUT,
            }),
            Arc::new(StubFactory),
            None,
            String::new(),
            None,
        ),
        Some((provider, model)) if model == "mock" => (
            Arc::new(StubPlanner {
                fanout: STUB_FANOUT,
            }),
            Arc::new(crew_hive::ApiFactory::new(provider, WORK_MAX_TOKENS)),
            None,
            model,
            None,
        ),
        Some((provider, model)) => {
            let planner: Arc<dyn Planner> = Arc::new(
                LlmPlanner {
                    provider: Arc::clone(&provider),
                    tier: ModelTier::Standard,
                    model: None,
                }
                .with_model(model.clone()),
            );
            let mut factory =
                crew_hive::ApiFactory::new(provider, WORK_MAX_TOKENS).with_model(model.clone());
            if let Some(t) = tools {
                factory = factory.with_tools(t);
            }
            (
                Arc::clone(&planner),
                Arc::new(factory),
                Some(Budget {
                    max_micros_usd: Budget::DEFAULT_MICROS_USD,
                }),
                model,
                Some(planner),
            )
        }
    }
}

/// Consume a pending `/resume` context (if any) and fold it into `task` as
/// restored context for the planner/execution prompt. The session log still
/// records the user's original, unfolded `task` text.
pub(super) fn fold_resume(session: &Session, task: &str) -> String {
    let resumed = session
        .resume
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    match resumed {
        Some(prev) => crate::broker::sessionlog::with_resume(&prev, task),
        None => task.to_string(),
    }
}

/// Transcript note for a telemetry overflow: the run finished, but `n`
/// events never reached the pane, so its per-task stats under-count.
pub(super) fn lagged_note(n: u64) -> String {
    format!(
        "telemetry gap: {n} event{} dropped (bus overflow) \u{2014} task stats may under-count",
        if n == 1 { "" } else { "s" }
    )
}
