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

    // Drain the bus and emit LIVE while the scheduler runs — join! interleaves
    // the three futures on this current-thread runtime, so each event reaches
    // the host as it happens instead of after the run (frozen-looking runs).
    let mut agent_task: HashMap<u64, TaskId> = HashMap::new();
    let mut tokens_total: u64 = 0;
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
                            for out in translate(&ev, &titles, &mut agent_task) {
                                emit(out)?;
                            }
                            Ok(())
                        });
                        if let Err(e) = r {
                            emit_err = Some(e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
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
        // A task failure is chat-visible content, not a connection loss: the
        // app's chat pane treats `PluginEvent::Error` as the broker connection
        // dropping (sets connected=false and discards the text), so surface
        // this as a normal message from the failing agent/task instead.
        HiveEvent::Failed { agent, error } => {
            vec![msg(
                agent_name(agent, agent_task).as_str(),
                format!("\u{2717} failed: {error}"),
            )]
        }
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

    // The merge (sink) task's output already streams live as its own
    // per-task Message the moment it completes (OutputChunk -> translate).
    // The final "swarm done" summary must not repeat that text — otherwise
    // the same answer appears twice back-to-back in the transcript.
    #[test]
    fn final_summary_does_not_repeat_sink_task_output() {
        let evs = collect("build the thing", Arc::new(AtomicBool::new(false)));
        // The merge task (id 2, depending on both leaves) streamed its own
        // output as a per-task message already.
        assert!(
            evs.iter()
                .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("deps=2"))),
            "expected the merge task's own streamed output message: {evs:?}"
        );
        // The closing summary must be status-only, not a repeat of that text.
        let summary = evs
            .iter()
            .rev()
            .find_map(|e| match e {
                PluginEvent::Message { text, .. } if text.contains("swarm done") => {
                    Some(text.clone())
                }
                _ => None,
            })
            .expect("expected a swarm done summary message");
        assert!(
            !summary.contains("deps="),
            "summary must not duplicate sink task output: {summary:?}"
        );
    }

    #[test]
    fn pre_cancelled_run_reports_cancellation() {
        let evs = collect("task", Arc::new(AtomicBool::new(true)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("cancelled"))));
    }

    // F1: a task failure must surface as chat-visible text, never as
    // `PluginEvent::Error` — the app's chat pane treats `Error` as the
    // broker connection dropping (sets connected=false, discards the text).
    #[test]
    fn task_failure_becomes_a_chat_message_not_a_connection_error() {
        use crew_hive::agent::FailingFactory;
        use crew_hive::TaskId;
        let mut fail_tasks = std::collections::HashSet::new();
        fail_tasks.insert(TaskId(0));
        let mut evs = Vec::new();
        run_with(
            "build the thing",
            Arc::new(StubPlanner { fanout: 2 }),
            Arc::new(FailingFactory { fail_tasks }),
            None,
            Arc::new(AtomicBool::new(false)),
            &mut |ev| {
                evs.push(ev);
                Ok(())
            },
        )
        .unwrap();
        assert!(
            evs.iter().any(|e| matches!(
                e,
                PluginEvent::Message { text, .. }
                    if text.contains("failed") && text.contains("stub failure")
            )),
            "expected a chat message surfacing the failure: {evs:?}"
        );
        assert!(
            !evs.iter().any(|e| matches!(e, PluginEvent::Error { .. })),
            "task failures must not be reported as PluginEvent::Error: {evs:?}"
        );
    }

    // F2: a pending `/resume` context must be consumed and folded into the
    // task the swarm path plans/executes, exactly like `relay_counting`
    // does for `@agent` tasks — otherwise restored context is silently
    // dropped by the default execution path.
    #[test]
    fn fold_resume_consumes_pending_context_once() {
        let session = Session::new();
        *session.resume.lock().unwrap() = Some("previous turn: it was the cache".into());

        let first = fold_resume(&session, "now fix the docs");
        assert!(first.contains("previous turn: it was the cache"));
        assert!(first.contains("now fix the docs"));
        assert!(first.to_uppercase().contains("PREVIOUS SESSION"));

        // Consumed: a second task sees no pending resume.
        let second = fold_resume(&session, "another task");
        assert_eq!(second, "another task");
    }

    // Live drain: events must reach `emit` WHILE the run executes, not be
    // buffered until the scheduler finishes. The merge task starts only
    // after both leaves complete — by then their events must have been
    // emitted already.
    #[test]
    fn events_are_emitted_during_the_run_not_after() {
        use crew_hive::agent::{Agent, AgentContext};
        use crew_hive::board::TaskResult;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;

        struct SnoopAgent {
            emitted: Arc<AtomicUsize>,
            merge_snapshot: Arc<Mutex<Option<usize>>>,
        }
        impl Agent for SnoopAgent {
            fn run(
                &self,
                ctx: AgentContext,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = TaskResult> + Send>>
            {
                let emitted = Arc::clone(&self.emitted);
                let snap = Arc::clone(&self.merge_snapshot);
                Box::pin(async move {
                    if !ctx.deps.is_empty() {
                        // The merge task: record how many events the host had
                        // received by the time it started.
                        *snap.lock().unwrap() = Some(emitted.load(Ordering::SeqCst));
                    }
                    let output = format!("snoop:{}", ctx.task.id.0);
                    ctx.bus.publish(crew_hive::HiveEvent::OutputChunk {
                        agent: ctx.agent.clone(),
                        text: output.clone(),
                    });
                    TaskResult {
                        task: ctx.task.id,
                        output,
                        success: true,
                    }
                })
            }
        }
        struct SnoopFactory {
            emitted: Arc<AtomicUsize>,
            merge_snapshot: Arc<Mutex<Option<usize>>>,
        }
        impl crew_hive::AgentFactory for SnoopFactory {
            fn make(&self, _kind: &crew_hive::AgentKind) -> Box<dyn Agent> {
                Box::new(SnoopAgent {
                    emitted: Arc::clone(&self.emitted),
                    merge_snapshot: Arc::clone(&self.merge_snapshot),
                })
            }
        }

        let emitted = Arc::new(AtomicUsize::new(0));
        let merge_snapshot = Arc::new(Mutex::new(None));
        let counter = Arc::clone(&emitted);
        run_with(
            "build the thing",
            Arc::new(StubPlanner { fanout: 2 }),
            Arc::new(SnoopFactory {
                emitted,
                merge_snapshot: Arc::clone(&merge_snapshot),
            }),
            None,
            Arc::new(AtomicBool::new(false)),
            &mut |_ev| {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap();
        let snap = merge_snapshot
            .lock()
            .unwrap()
            .expect("merge task must have run");
        assert!(
            snap > 2, // more than just HivePlan + plan-summary message
            "leaf events must be emitted before the merge task starts (got {snap})"
        );
    }

    // F4: the run emits one aggregate Stats event (turn-total: empty agent)
    // summing the drained TokenDelta events, before the final summary
    // message closes the run, so the chat header's token/cost meter isn't
    // left empty for swarm runs.
    #[test]
    fn run_emits_an_aggregate_stats_event_with_tokens_and_exchange_count() {
        let evs = collect("build the thing", Arc::new(AtomicBool::new(false)));
        let stats = evs.iter().find_map(|e| match e {
            PluginEvent::Stats {
                exchanges,
                tokens,
                agent,
                ..
            } => Some((*exchanges, *tokens, agent.clone())),
            _ => None,
        });
        let (exchanges, tokens, agent) = stats.expect("expected an aggregate Stats event");
        assert!(
            tokens > 0,
            "stub agents emit TokenDelta so the aggregate should be > 0"
        );
        assert_eq!(exchanges, 3, "3 stub tasks complete (2 leaves + merge)");
        assert!(
            agent.is_empty(),
            "empty agent = turn-total per protocol.rs Stats docs"
        );
        let stats_pos = evs
            .iter()
            .position(|e| matches!(e, PluginEvent::Stats { .. }))
            .unwrap();
        let summary_pos = evs
            .iter()
            .rposition(
                |e| matches!(e, PluginEvent::Message { text, .. } if text.contains("swarm done")),
            )
            .unwrap();
        assert!(
            stats_pos < summary_pos,
            "Stats must land before the final summary message"
        );
    }
}
