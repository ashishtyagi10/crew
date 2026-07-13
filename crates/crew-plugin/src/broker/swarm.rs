//! Default /crew execution: plan a plain message into a crew-hive task
//! graph and run it as a swarm on this worker thread, streaming chat
//! events plus raw Hive telemetry for the host's companion graph pane.
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

/// $1.00 ceiling per swarm run, enforced by the budget governor.
const SWARM_BUDGET_MICROS_USD: u64 = 1_000_000;
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
                max_micros_usd: SWARM_BUDGET_MICROS_USD,
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
    super::sessionlog::append("user", task);
    let (planner, factory, budget) = backend();
    run_with(
        task,
        planner,
        factory,
        budget,
        Arc::clone(&session.cancel),
        emit,
    )
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
            };
            TaskGraph::new(vec![single]).expect("single task graph is valid")
        }
    };

    let tasks: Vec<crew_hive::TaskSpec> = graph.tasks().to_vec();
    let titles: HashMap<TaskId, String> = tasks.iter().map(|t| (t.id, t.title.clone())).collect();
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

    // Execute: scheduler + optional budget governor + bus drain, all on this
    // thread's runtime (the pattern proven in crew-app/src/swarm/bridge.rs).
    let board = Blackboard::new();
    let bus = EventBus::new(256);
    let mut sub = bus.subscribe();
    let governor = budget.map(|b| budget_governor(bus.clone(), b, Arc::clone(&cancel)));
    let sched = Scheduler::new(graph.clone(), board.clone(), bus, factory, CONCURRENCY)
        .with_cancel(Arc::clone(&cancel));

    let (tx, rx) = std::sync::mpsc::channel::<HiveEvent>();
    let outcome = rt.block_on(async move {
        let drain = async move {
            while let Ok(ev) = sub.recv().await {
                if tx.send(ev).is_err() {
                    break;
                }
            }
        };
        match governor {
            Some(g) => tokio::join!(sched.run(), drain, g).0,
            None => tokio::join!(sched.run(), drain).0,
        }
    });

    // Translate the drained telemetry (the run has completed; the channel
    // holds the full ordered stream).
    let mut agent_task: HashMap<u64, TaskId> = HashMap::new();
    while let Ok(ev) = rx.try_recv() {
        emit(PluginEvent::Hive { event: ev.clone() })?;
        for out in translate(&ev, &titles, &mut agent_task) {
            emit(out)?;
        }
    }

    // Final aggregate: sink tasks' outputs (tasks nothing depends on).
    let sink_ids: Vec<TaskId> = tasks
        .iter()
        .filter(|t| !tasks.iter().any(|o| o.deps.contains(&t.id)))
        .map(|t| t.id)
        .collect();
    let sinks = rt.block_on(board.gather(&sink_ids));
    let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);
    let mut summary = if cancelled {
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
    for r in &sinks {
        if r.success && !r.output.is_empty() {
            summary.push_str("\n\n");
            summary.push_str(&r.output);
        }
    }
    emit(msg("crew", summary))?;
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    Ok(())
}

/// Map one HiveEvent to chat-facing events. Raw `Hive` forwarding happens at
/// the call site; this returns only the human-readable translations.
fn translate(
    ev: &HiveEvent,
    titles: &HashMap<TaskId, String>,
    agent_task: &mut HashMap<u64, TaskId>,
) -> Vec<PluginEvent> {
    let title_of = |t: &TaskId| {
        titles
            .get(t)
            .cloned()
            .unwrap_or_else(|| format!("task-{}", t.0))
    };
    let agent_name = |a: &AgentId, agent_task: &HashMap<u64, TaskId>| {
        agent_task
            .get(&a.0)
            .map(title_of)
            .unwrap_or_else(|| format!("agent-{}", a.0))
    };
    match ev {
        HiveEvent::AgentSpawned { agent, task } => {
            agent_task.insert(agent.0, *task);
            vec![PluginEvent::Activity {
                agent: title_of(task),
                state: "thinking".into(),
                from: "hive".into(),
            }]
        }
        HiveEvent::TaskStateChanged { task, state } => match state {
            TaskState::Done | TaskState::Failed | TaskState::Cancelled => {
                vec![PluginEvent::Activity {
                    agent: title_of(task),
                    state: "idle".into(),
                    from: String::new(),
                }]
            }
            _ => vec![],
        },
        HiveEvent::TokenDelta { agent, output, .. } => vec![PluginEvent::StatsTick {
            agent: agent_name(agent, agent_task),
            tokens: u64::from(*output),
        }],
        HiveEvent::CostDelta { .. } => vec![],
        HiveEvent::OutputChunk { agent, text } => {
            vec![msg(agent_name(agent, agent_task).as_str(), text.clone())]
        }
        HiveEvent::Failed { agent, error } => vec![PluginEvent::Error {
            message: format!("{}: {error}", agent_name(agent, agent_task)),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crew_hive::agent::StubFactory;
    use crew_hive::StubPlanner;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn collect(task: &str, cancel: Arc<AtomicBool>) -> Vec<PluginEvent> {
        let mut evs = Vec::new();
        run_with(
            task,
            Arc::new(StubPlanner { fanout: 2 }),
            Arc::new(StubFactory),
            None,
            cancel,
            &mut |ev| {
                evs.push(ev);
                Ok(())
            },
        )
        .unwrap();
        evs
    }

    #[test]
    fn plain_task_emits_plan_then_hive_events_then_summary() {
        let evs = collect("build the thing", Arc::new(AtomicBool::new(false)));
        // A HivePlan with 3 tasks (2 leaves + merge) is announced first.
        assert!(matches!(
            evs.first(),
            Some(PluginEvent::HivePlan { tasks }) if tasks.len() == 3
        ));
        // Raw telemetry flows for the graph pane.
        assert!(evs.iter().any(|e| matches!(e, PluginEvent::Hive { .. })));
        // A plan-summary chat message names the tasks.
        assert!(evs
            .iter()
            .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("leaf-0"))));
        // The final aggregate message closes the run.
        assert!(evs.iter().rev().any(
            |e| matches!(e, PluginEvent::Message { text, .. } if text.contains("swarm done"))
        ));
    }

    #[test]
    fn pre_cancelled_run_reports_cancellation() {
        let evs = collect("task", Arc::new(AtomicBool::new(true)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("cancelled"))));
    }
}
