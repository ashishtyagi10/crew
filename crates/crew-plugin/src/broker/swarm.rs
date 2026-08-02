//! Default /crew execution: plan a plain message into a crew-hive task
//! graph and run it as a swarm on this worker thread, streaming chat
//! events plus raw Hive telemetry live — as the scheduler runs, not
//! buffered until it completes — for the host's companion graph pane.
//! `@agent` addressing bypasses this module (stdio routes it to the relay).
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crew_hive::{
    budget_governor, AgentFactory, AgentId, Blackboard, Budget, EventBus, HiveEvent, ModelTier,
    Planner, Scheduler, TaskGraph, TaskId, TaskState,
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

/// Entry point for a plain (unaddressed) chat task.
pub(crate) fn run_task(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // A pending resume folds the previous session's tail in as restored
    // context (consumed once) — mirroring `relay_counting` — and skills
    // weave in first, matched on the raw task, exactly as on the relay path.
    let task_owned = fold_resume(session, &super::skillframe::with_skills(task));
    super::sessionlog::append("user", task);
    let (planner, factory, budget, model, replan) = backend();
    run_with(
        &task_owned,
        planner,
        factory,
        budget,
        &model,
        Arc::clone(&session.cancel),
        replan,
        emit,
    )
}

/// Injectable core: plan `task`, execute the graph, translate events.
/// `model` is the slug serving this run's API agents (empty when unknown —
/// stub/keyless runs); it stamps the re-emitted roster so the host's footer
/// can show what is serving right now. `replan`, when set (real-provider
/// runs — see `swarmconf::backend`), lets the scheduler re-plan the
/// remainder once on the first task failure.
#[allow(clippy::too_many_arguments)] // the run's full configuration, injected by tests piecewise
pub(crate) fn run_with(
    task: &str,
    planner: Arc<dyn Planner>,
    factory: Arc<dyn AgentFactory>,
    budget: Option<Budget>,
    model: &str,
    cancel: Arc<AtomicBool>,
    replan: Option<Arc<dyn Planner>>,
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
                "agent smith",
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
        "agent smith",
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
    // The roster leads with the run's own cast, stamped with the model
    // serving it, built from memory: `record` above is best-effort (a broker
    // launched from Finder/Dock runs at `/`, where `.crew/` is unwritable),
    // and a disk re-read would come back empty exactly then — taking the
    // footer's model segment with it. Discovery still appends everyone the
    // cast doesn't name (CLI agents, manifest plugins, stored specialists).
    let mut agents: Vec<crate::AgentInfo> = seen
        .iter()
        .map(|(name, role)| crate::AgentInfo {
            name: name.clone(),
            role: role.clone(),
            model: model.to_string(),
        })
        .collect();
    for info in super::Registry::discover().infos() {
        if !agents.iter().any(|a| a.name == info.name) {
            agents.push(info);
        }
    }
    emit(PluginEvent::Roster { agents })?;

    // Execute: scheduler + optional budget governor + bus drain, all on this
    // thread's runtime (the pattern proven in crew-app/src/swarm/bridge.rs).
    let board = Blackboard::new();
    let bus = EventBus::new(EventBus::DEFAULT_CAPACITY);
    let mut sub = bus.subscribe();
    let governor = budget.map(|b| budget_governor(bus.clone(), b, Arc::clone(&cancel)));
    let mut sched = Scheduler::new(graph.clone(), board.clone(), bus, factory, CONCURRENCY)
        .with_cancel(Arc::clone(&cancel));
    if let Some(rp) = replan {
        sched = sched.with_replan(task, rp);
    }

    // Drain the bus and emit LIVE while the scheduler runs — join! interleaves
    // the three futures on this current-thread runtime, so each event reaches
    // the host as it happens instead of after the run (frozen-looking runs).
    let mut agent_task: HashMap<u64, TaskId> = HashMap::new();
    // One TextGate per agent id, plus the run clock they pace against —
    // `translate` has neither, so both are threaded in (see its doc comment).
    let mut gates: HashMap<u64, crate::broker::tick::TextGate> = HashMap::new();
    let run_start = std::time::Instant::now();
    let mut tokens_total: u64 = 0;
    let mut in_total: u64 = 0;
    let mut out_total: u64 = 0;
    let mut cost_total: u64 = 0;
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
                        match &ev {
                            HiveEvent::TokenDelta { input, output, .. } => {
                                let in_count = u64::from(*input);
                                let out_count = u64::from(*output);
                                tokens_total += in_count + out_count;
                                in_total += in_count;
                                out_total += out_count;
                            }
                            HiveEvent::CostDelta { micros_usd, .. } => {
                                cost_total += micros_usd;
                            }
                            _ => {}
                        }
                        // `OutputDelta` fires once per SSE fragment (see
                        // `ApiAgent`), so forwarding it raw here would flood
                        // the wire with exactly what `TextGate` (tick.rs)
                        // exists to coalesce — and the app ignores raw
                        // `Hive{OutputDelta}` outright (chatswarm.rs's no-op
                        // arm), so every one of those lines is pure waste.
                        // Only the coalesced `Delta` that `translate` derives
                        // from it may cross the wire. Every other variant
                        // still forwards raw below, unaffected — do not
                        // widen this exclusion.
                        let mut r = if matches!(ev, HiveEvent::OutputDelta { .. }) {
                            Ok(())
                        } else {
                            emit(PluginEvent::Hive { event: ev.clone() })
                        };
                        r = r.and_then(|()| {
                            for out in translate(
                                &ev,
                                &specialties,
                                &mut agent_task,
                                &mut gates,
                                run_start.elapsed().as_millis() as u64,
                            ) {
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
        emit(msg("agent smith", lagged_note(lagged_total)))?;
    }

    // Final aggregate: a status line only. Sink tasks' outputs already
    // streamed live as their own per-task Messages the moment they completed
    // (OutputChunk -> `translate` -> `msg`), so repeating them here would
    // duplicate the same answer back-to-back in the transcript.
    // A clean run says nothing — the sink tasks' answers already streamed
    // live, so a "swarm done" line is just chrome. Only a cancellation or a
    // failure gets an aggregate note, since those aren't otherwise obvious.
    let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);
    let summary = if cancelled {
        Some(format!(
            "swarm cancelled (budget or /stop) — {} done, {} failed, {} cancelled",
            outcome.done.len(),
            outcome.failed.len(),
            outcome.cancelled.len()
        ))
    } else if !outcome.failed.is_empty() {
        Some(format!(
            "swarm finished with {} failed task(s)",
            outcome.failed.len()
        ))
    } else {
        None
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
        tok_in: in_total,
        tok_out: out_total,
        cost_microusd: cost_total,
    })?;
    if let Some(summary) = summary {
        emit(msg("agent smith", summary))?;
    }
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    Ok(())
}

#[path = "swarmconf.rs"]
mod swarmconf;
use swarmconf::{backend, fold_resume, lagged_note};

#[path = "swarmmsg.rs"]
mod swarmmsg;
use swarmmsg::translate;

#[cfg(test)]
#[path = "swarm_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "swarmreplan_tests.rs"]
mod replan_tests;
