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

/// Per-task output token cap for worker agents.
const WORK_MAX_TOKENS: u32 = 2048;
/// Fan-out for the offline stub planner.
const STUB_FANOUT: usize = 2;
/// The name a run-level `Loaded` event carries — the plan line's own sender.
const SWARM_LEAD: &str = "agent smith";

/// Entry point for a plain (unaddressed) chat task. `verify` is the router's
/// `VERIFY: yes`: the result is judged against the request when it ends.
pub(crate) fn run_task(
    task: &str,
    verify: bool,
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
    // The lead's closing call and the judge run on routing's gates: keyless
    // and mock runs get neither, and the pane sees exactly what it saw before.
    let synth = swarmanswer::live();
    let judge = swarmverify::live(verify);
    run_with_synth(
        &task_owned,
        planner,
        factory,
        budget,
        &model,
        Arc::clone(&session.cancel),
        replan,
        synth.as_deref(),
        judge.as_deref().map(swarmverify::Judge::new),
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
        task, planner, factory, budget, model, cancel, replan, None, None, emit,
    )
}

/// Injectable core: plan `task`, execute the graph, translate events.
/// `model` is the slug serving this run's API agents (empty when unknown —
/// stub/keyless runs); it stamps the re-emitted roster so the host's footer
/// can show what is serving right now. `replan`, when set (real-provider
/// runs — see `swarmconf::backend`), lets the scheduler re-plan the
/// remainder once on the first task failure. `synth` is the lead's closing
/// call (`swarmanswer`): `None` means no answer line, ever. `verify` is the
/// judge (`swarmverify`): `None` means the run ends unjudged; a `NOT MET`
/// verdict runs this same function once more on the revision it names.
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
    verify: swarmverify::Verify<'_>,
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
    swarmcast::announce(&tasks, model, emit)?;

    // Execute: scheduler + optional budget governor + bus drain, all on this
    // thread's runtime (the pattern proven in crew-app/src/swarm/bridge.rs).
    let board = Blackboard::new();
    let bus = EventBus::new(EventBus::DEFAULT_CAPACITY);
    let mut sub = bus.subscribe();
    let governor = budget.map(|b| budget_governor(bus.clone(), b, Arc::clone(&cancel)));
    // The width follows the plan (see `swarmwidth`), not a constant.
    let width = swarmwidth::concurrency_for(&graph);
    let mut sched = Scheduler::new(
        graph.clone(),
        board.clone(),
        bus,
        Arc::clone(&factory),
        width,
    )
    .with_cancel(Arc::clone(&cancel));
    if let Some(rp) = &replan {
        sched = sched.with_replan(task, Arc::clone(rp));
    }

    // Drain the bus and emit LIVE while the scheduler runs — join! interleaves
    // the three futures on this current-thread runtime, so each event reaches
    // the host as it happens instead of after the run (frozen-looking runs).
    let mut agent_task: HashMap<u64, TaskId> = HashMap::new();
    // One TextGate per agent id, plus the run clock they pace against —
    // `translate` has neither, so both are threaded in (see its doc comment).
    let mut gates: HashMap<u64, crate::broker::tick::TextGate> = HashMap::new();
    let run_start = std::time::Instant::now();
    let mut tally = swarmtally::Tally::new(tasks.len());
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
                        // Sums for the aggregate Stats, and the one note the
                        // tally speaks (the tool pool ran dry) — said BEFORE
                        // the event that emptied it is forwarded, so the
                        // pane reads the why above the refused call.
                        let mut r = match tally.observe(&ev) {
                            Some(note) => emit(note),
                            None => Ok(()),
                        };
                        // Which variants cross the wire, and how a tool
                        // result is bounded, is `swarmmsg::forwarded`'s call.
                        r = r.and_then(|()| match swarmmsg::forwarded(&ev) {
                            Some(event) => emit(PluginEvent::Hive { event }),
                            None => Ok(()),
                        });
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
    // sinks' own replies are not already it (`swarmanswer` decides), and
    // then the verdict when a judge sits (`swarmverify`); on a cancellation
    // or a failure it is the status line, since neither is otherwise
    // obvious. Never a "swarm done": that would be chrome.
    let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);
    let mut revise = None;
    if !cancelled && outcome.failed.is_empty() {
        let results = rt.block_on(board.gather(&outcome.done));
        let answer = swarmanswer::combine(task, &graph, &results, synth, emit)?;
        if let Some(judge) = verify {
            let answer = answer.as_deref();
            revise = swarmverify::verdict(task, &graph, &results, answer, judge, emit)?;
        }
    }
    let summary = swarmanswer::closing_line(&outcome, cancelled);
    // One aggregate Stats for the whole run (empty `agent` = turn-total, per
    // the field docs in protocol.rs) so the chat header's token/cost meter
    // and stdio's per-task counter aren't left empty for swarm runs.
    emit(PluginEvent::Stats {
        exchanges: outcome.done.len() as u32,
        tokens: tally.tokens,
        agent: String::new(),
        ms: 0,
        ctx: 0,
        tok_in: tally.tok_in,
        tok_out: tally.tok_out,
        cost_microusd: tally.cost,
        tools: Some(outcome.tool_rounds),
    })?;
    if let Some(summary) = summary {
        emit(msg("agent smith", summary))?;
    }
    // A `NOT MET` verdict sends the crew back once, through the same planner,
    // factory and closing call; that pass settles the turn's activity itself.
    if let Some((revised, verify)) = revise {
        return run_with_synth(
            &revised, planner, factory, budget, model, cancel, replan, synth, verify, emit,
        );
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

#[path = "swarmcast.rs"]
mod swarmcast;

#[path = "swarmverify.rs"]
mod swarmverify;

#[path = "swarmwidth.rs"]
mod swarmwidth;

#[path = "swarmtally.rs"]
mod swarmtally;

#[cfg(test)]
#[path = "swarm_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "swarmbudget_tests.rs"]
mod budget_tests;

#[cfg(test)]
#[path = "swarmreplan_tests.rs"]
mod replan_tests;

#[cfg(test)]
#[path = "swarmmemory_tests.rs"]
mod memory_tests;
