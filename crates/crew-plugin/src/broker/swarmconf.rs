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

/// The sidecar command, if one is configured AND runnable. `CREW_SIDECAR` is a command line —
/// `python3 -m crew_langgraph` — and it is opt-in in every direction: unset by default, probed
/// before it is spawned, and reported by `/doctor` either way.
pub(super) fn sidecar_command() -> Option<(String, Vec<String>)> {
    let raw = std::env::var("CREW_SIDECAR").ok()?;
    let (program, args) = crew_hive::worker::stdio::parse_command(&raw)?;
    crew_hive::worker::stdio::probe(&program).then_some((program, args))
}

/// The sidecar factory, when one is configured, runnable, and starts.
///
/// A failure to spawn falls back to crew's own agents rather than failing the task: the sidecar
/// is an engine BEHIND the bridge, never crew's spine, and a machine that lost its Python must
/// go on working exactly as it did before. It says so on stderr, where the broker's log is.
fn sidecar_factory(
    tools: &Option<Arc<dyn crew_hive::tools::Tools>>,
) -> Option<Arc<dyn AgentFactory>> {
    let (program, args) = sidecar_command()?;
    match crew_hive::worker::stdio::StdioTransport::spawn(&program, &args) {
        Ok(t) => Some(Arc::new(
            crew_hive::RemoteFactory::new(Arc::new(t)).with_tools(tools.clone()),
        )),
        Err(e) => {
            eprintln!("crew: could not start the sidecar `{program}`: {e} — running natively");
            None
        }
    }
}

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
                    capabilities: Vec::new(),
                }
                .with_model(model.clone())
                // The planner runs once, before there are tasks, so it never sees a tool
                // hint; this is how it learns a goal is reachable at all.
                .with_capabilities(tools.as_ref().map(|t| t.capabilities()).unwrap_or_default()),
            );
            // The sidecar replaces the AGENTS, never the planner: crew decomposes the goal and
            // owns the tools and the gate, and the engine behind the bridge runs the tasks.
            let factory: Arc<dyn AgentFactory> = match sidecar_factory(&tools) {
                Some(f) => f,
                None => {
                    let mut f = crew_hive::ApiFactory::new(provider, WORK_MAX_TOKENS)
                        .with_model(model.clone());
                    if let Some(t) = tools {
                        f = f.with_tools(t);
                    }
                    Arc::new(f)
                }
            };
            (
                Arc::clone(&planner),
                factory,
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

#[cfg(test)]
#[path = "swarmconf_tests.rs"]
mod tests;
