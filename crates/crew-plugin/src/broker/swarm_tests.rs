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
    assert!(evs
        .iter()
        .rev()
        .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("swarm done"))));
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
            PluginEvent::Message { text, .. } if text.contains("swarm done") => Some(text.clone()),
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
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = TaskResult> + Send>> {
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
