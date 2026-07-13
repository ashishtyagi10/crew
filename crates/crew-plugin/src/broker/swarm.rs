//! Default /crew execution: plan a plain message into a crew-hive task
//! graph and run it as a swarm on this worker thread, streaming chat
//! events plus raw Hive telemetry live — as the scheduler runs, not
//! buffered until it completes — for the host's companion graph pane.
//! `@agent` addressing bypasses this module (stdio routes it to the relay).
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crew_hive::agent::StubFactory;
use crew_hive::{
    budget_governor, AgentFactory, AgentId, Blackboard, Budget, EventBus, HiveEvent, LlmPlanner,
    ModelTier, Planner, Scheduler, StubPlanner, TaskGraph, TaskId, TaskState,
};

use crate::protocol::PluginEvent;

use super::relay::msg;
use super::session::Session;

/// Parallel worker agents per run.
const CONCURRENCY: usize = 4;
/// Per-task output token cap for worker agents.
const WORK_MAX_TOKENS: u32 = 2048;
/// Fan-out for the offline stub planner.
const STUB_FANOUT: usize = 2;

/// Pick planner/factory/budget from provider discovery: real LLM planning on
/// a discovered provider; deterministic stubs when keyless. The mock provider
/// (GUI harness) plans with stubs but executes through the mock, so replies
/// stay deterministic while the full pipeline runs.
fn backend() -> (Arc<dyn Planner>, Arc<dyn AgentFactory>, Option<Budget>) {
    match super::discover::provider_and_model() {
        None => (
            Arc::new(StubPlanner {
                fanout: STUB_FANOUT,
            }),
            Arc::new(StubFactory),
            None,
        ),
        Some((provider, model)) if model == "mock" => (
            Arc::new(StubPlanner {
                fanout: STUB_FANOUT,
            }),
            Arc::new(crew_hive::ApiFactory::new(provider, WORK_MAX_TOKENS)),
            None,
        ),
        Some((provider, model)) => (
            Arc::new(
                LlmPlanner {
                    provider: Arc::clone(&provider),
                    tier: ModelTier::Standard,
                    model: None,
                }
                .with_model(model.clone()),
            ),
            Arc::new(crew_hive::ApiFactory::new(provider, WORK_MAX_TOKENS).with_model(model)),
            Some(Budget {
                max_micros_usd: Budget::DEFAULT_MICROS_USD,
            }),
        ),
    }
}

/// Entry point for a plain (unaddressed) chat task.
pub(crate) fn run_task(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // A `/resume` before this task folds the previous session's tail in as
    // restored context (consumed once) — mirrors `relay_counting`'s handling
    // of `@agent` tasks so the default swarm path doesn't silently ignore it.
    let task_owned = fold_resume(session, task);
    super::sessionlog::append("user", task);
    let (planner, factory, budget) = backend();
    run_with(
        &task_owned,
        planner,
        factory,
        budget,
        Arc::clone(&session.cancel),
        emit,
    )
}

/// Consume a pending `/resume` context (if any) and fold it into `task` as
/// restored context for the planner/execution prompt. The session log still
/// records the user's original, unfolded `task` text.
fn fold_resume(session: &Session, task: &str) -> String {
    let resumed = session
        .resume
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    match resumed {
        Some(prev) => super::sessionlog::with_resume(&prev, task),
        None => task.to_string(),
    }
}

/// Injectable core: plan `task`, execute the graph, translate events.
pub(crate) fn run_with(
    task: &str,
    planner: Arc<dyn Planner>,
    factory: Arc<dyn AgentFactory>,
    budget: Option<Budget>,
    cancel: Arc<AtomicBool>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // NB: nothing is emitted before `HivePlan` so the host opens its companion
    // graph pane on the very first event of a swarm run (see `run_with` tests).
    let graph = match rt.block_on(planner.plan(task)) {
        Ok(g) => g,
        Err(e) => {
            emit(msg(
                "crew",
                format!("planning failed ({e}) — answering directly"),
            ))?;
            emit(PluginEvent::Activity {
                agent: String::new(),
                state: "idle".into(),
                from: String::new(),
            })?;
            // Degrade to a single-task graph so chat never dead-ends.
            let single = crew_hive::TaskSpec {
                id: TaskId(0),
                title: "reply".into(),
                agent: crew_hive::AgentKind::Api { system: None },
                model: ModelTier::Standard,
                deps: vec![],
                prompt: task.to_owned(),
                specialty: String::new(),
                expertise: String::new(),
            };
            TaskGraph::new(vec![single]).expect("single task graph is valid")
        }
    };

    let tasks: Vec<crew_hive::TaskSpec> = graph.tasks().to_vec();
    // Titles are not collected here: `HivePlan` already carries them to the
    // app, and `translate` names agents by specialty. Handing it titles too is
    // what let an agent be named after its task.
    let specialties: HashMap<TaskId, String> =
        tasks.iter().map(|t| (t.id, t.specialty.clone())).collect();
    emit(PluginEvent::HivePlan {
        tasks: tasks.clone(),
    })?;
    emit(msg(
        "crew",
        format!(
            "planned {} task(s): {}",
            tasks.len(),
            tasks
                .iter()
                .map(|t| t.title.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))?;

    // Persist this run's cast, then re-emit the roster: `Roster` is otherwise
    // only sent from `hello()`, so without this the app never learns about a
    // specialist invented mid-session and the new names never appear.
    // First-wins on a duplicate name: one name is one specialist.
    let mut seen: Vec<(String, String)> = Vec::new();
    for t in &tasks {
        if !seen.iter().any(|(n, _)| n == &t.specialty) {
            seen.push((t.specialty.clone(), t.expertise.clone()));
        }
    }
    super::specialists::record(&seen);
    emit(PluginEvent::Roster {
        agents: super::Registry::discover().infos(),
    })?;

    // Execute: scheduler + optional budget governor + bus drain, all on this
    // thread's runtime (the pattern proven in crew-app/src/swarm/bridge.rs).
    let board = Blackboard::new();
    let bus = EventBus::new(EventBus::DEFAULT_CAPACITY);
    let mut sub = bus.subscribe();
    let governor = budget.map(|b| budget_governor(bus.clone(), b, Arc::clone(&cancel)));
    let sched = Scheduler::new(graph.clone(), board.clone(), bus, factory, CONCURRENCY)
        .with_cancel(Arc::clone(&cancel));

    // Drain the bus and emit LIVE while the scheduler runs — join! interleaves
    // the three futures on this current-thread runtime, so each event reaches
    // the host as it happens instead of after the run (frozen-looking runs).
    let mut agent_task: HashMap<u64, TaskId> = HashMap::new();
    let mut tokens_total: u64 = 0;
    let mut lagged_total: u64 = 0;
    let mut emit_err: Option<anyhow::Error> = None;
    let outcome = rt.block_on(async {
        let drain = async {
            loop {
                match sub.recv().await {
                    Ok(ev) => {
                        if emit_err.is_some() {
                            continue; // keep consuming so the scheduler finishes
                        }
                        if let HiveEvent::TokenDelta { input, output, .. } = &ev {
                            tokens_total += u64::from(*input) + u64::from(*output);
                        }
                        let r = emit(PluginEvent::Hive { event: ev.clone() }).and_then(|()| {
                            for out in translate(&ev, &specialties, &mut agent_task) {
                                emit(out)?;
                            }
                            Ok(())
                        });
                        if let Err(e) = r {
                            emit_err = Some(e);
                        }
                    }
                    // Skipping keeps the run alive, but the skips must not
                    // be silent: per-task tokens/cost under-count after a
                    // gap, and the user should know why.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        lagged_total += n;
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        match governor {
            Some(g) => tokio::join!(sched.run(), drain, g).0,
            None => tokio::join!(sched.run(), drain).0,
        }
    });
    if let Some(e) = emit_err {
        return Err(e);
    }
    if lagged_total > 0 {
        emit(msg("crew", &lagged_note(lagged_total)))?;
    }

    // Final aggregate: a status line only. Sink tasks' outputs already
    // streamed live as their own per-task Messages the moment they completed
    // (OutputChunk -> `translate` -> `msg`), so repeating them here would
    // duplicate the same answer back-to-back in the transcript.
    let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);
    let summary = if cancelled {
        format!(
            "swarm cancelled (budget or /stop) — {} done, {} failed, {} cancelled",
            outcome.done.len(),
            outcome.failed.len(),
            outcome.cancelled.len()
        )
    } else {
        format!(
            "swarm done — {} task(s), {} failed",
            outcome.done.len(),
            outcome.failed.len()
        )
    };
    // One aggregate Stats for the whole run (empty `agent` = turn-total, per
    // the field docs in protocol.rs) so the chat header's token/cost meter
    // and stdio's per-task counter aren't left empty for swarm runs.
    emit(PluginEvent::Stats {
        exchanges: outcome.done.len() as u32,
        tokens: tokens_total,
        agent: String::new(),
        ms: 0,
        ctx: 0,
    })?;
    emit(msg("crew", summary))?;
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    Ok(())
}

/// Transcript note for a telemetry overflow: the run finished, but `n`
/// events never reached the pane, so its per-task stats under-count.
fn lagged_note(n: u64) -> String {
    format!(
        "telemetry gap: {n} event{} dropped (bus overflow) \u{2014} task stats may under-count",
        if n == 1 { "" } else { "s" }
    )
}

#[path = "swarmmsg.rs"]
mod swarmmsg;
use swarmmsg::translate;

#[cfg(test)]
#[path = "swarm_tests.rs"]
mod tests;
