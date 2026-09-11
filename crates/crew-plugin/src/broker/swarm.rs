//! Default /crew execution: plan a plain message into a crew-hive task
//! graph and run it as a swarm on this worker thread, streaming chat
//! events plus raw Hive telemetry live — as the scheduler runs, not
//! buffered until it completes — for the host's companion graph pane.
//! `@agent` addressing bypasses this module (stdio routes it to the relay).
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crew_hive::{
    budget_governor, AgentFactory, AgentId, Blackboard, Budget, EventBus, HiveEvent, Planner,
    Scheduler, TaskId, TaskState,
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
/// The name a run-level `Loaded` event carries — the plan line's own sender.
const SWARM_LEAD: &str = "agent smith";

/// Entry point for a plain (unaddressed) chat task.
pub(crate) fn run_task(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // Skills weave in first (matched on the raw task), standing memory rides
    // on top — the `relay_turn` frame, so a `#note` reaches the planner on
    // the shape most messages take — and a pending resume folds in last,
    // consumed once. Each applied playbook is announced under the run's
    // lead, "agent smith": the sender of the plan line the pane anchors it above.
    let framed = super::skillframe::with_skills(task);
    for ev in super::skillframe::loaded_events(&framed.applied, SWARM_LEAD) {
        emit(ev)?;
    }
    let task_owned = fold_resume(session, &super::memory::with_memory(&framed.body));
    super::sessionlog::append("user", task);
    let (planner, factory, budget, model, replan) = backend(session.tools());
    // The lead's closing call runs on routing's gates: keyless and mock runs
    // get no call, and the pane sees exactly what it saw before.
    let synth = swarmanswer::live();
    run_with_synth(
        &task_owned,
        planner,
        factory,
        budget,
        &model,
        Arc::clone(&session.cancel),
        replan,
        synth.as_deref(),
        emit,
    )
}

/// [`run_with_synth`] with no closing call — the keyless shape, and the one
/// every test that pins the per-task event stream drives. Test-only because
/// production always goes through `run_task`, which decides the call itself.
#[cfg(test)]
#[allow(clippy::too_many_arguments)] // see `run_with_synth`
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
    run_with_synth(
        task, planner, factory, budget, model, cancel, replan, None, emit,
    )
}

/// Injectable core: plan `task`, execute the graph, translate events.
/// `model` is the slug serving this run's API agents (empty when unknown —
/// stub/keyless runs); it stamps the re-emitted roster so the host's footer
/// can show what is serving right now. `replan`, when set (real-provider
/// runs — see `swarmconf::backend`), lets the scheduler re-plan the
/// remainder once on the first task failure. `synth` is the lead's closing
/// call (`swarmanswer`): `None` means no answer line, ever.
#[allow(clippy::too_many_arguments)] // the run's full configuration, injected by tests piecewise
pub(crate) fn run_with_synth(
    task: &str,
    planner: Arc<dyn Planner>,
    factory: Arc<dyn AgentFactory>,
    budget: Option<Budget>,
    model: &str,
    cancel: Arc<AtomicBool>,
    replan: Option<Arc<dyn Planner>>,
    synth: swarmanswer::Synth<'_>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // NB: nothing is emitted before `HivePlan` so the host opens its companion
    // graph pane on the very first event of a swarm run (see `run_with` tests).
    let graph = match rt.block_on(planner.plan(task)) {
        Ok(g) => g,
        Err(e) => degraded(task, &e, emit)?,
    };

    let tasks: Vec<crew_hive::TaskSpec> = graph.tasks().to_vec();
    // Titles are not collected here: `HivePlan` already carries them, and
    // handing `translate` titles is what let an agent be named after its task.
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
    emit(super::rosterev::roster(agents))?;

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
                        // Which variants cross the wire, and how a tool
                        // result is bounded, is `swarmmsg::forwarded`'s call.
                        let mut r = match swarmmsg::forwarded(&ev) {
                            Some(event) => emit(PluginEvent::Hive { event }),
                            None => Ok(()),
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

    // The lead's closing word. On a clean run it is the ONE answer, when the
    // sinks' own replies are not already it (`swarmanswer` decides); on a
    // cancellation or a failure it is the status line, since neither is
    // otherwise obvious. Never a "swarm done": that would be chrome.
    let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);
    if !cancelled && outcome.failed.is_empty() {
        let results = rt.block_on(board.gather(&outcome.done));
        swarmanswer::combine(task, &graph, &results, synth, emit)?;
    }
    let summary = swarmanswer::closing_line(&outcome, cancelled);
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
use swarmconf::{backend, degraded, fold_resume, lagged_note};

#[path = "swarmmsg.rs"]
mod swarmmsg;
use swarmmsg::translate;

#[path = "swarmanswer.rs"]
mod swarmanswer;

#[cfg(test)]
#[path = "swarm_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "swarmreplan_tests.rs"]
mod replan_tests;

#[cfg(test)]
#[path = "swarmmemory_tests.rs"]
mod memory_tests;
