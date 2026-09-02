use super::*;
use crate::broker::testenv;
use crate::broker::tick::TextGate;
use crew_hive::agent::StubFactory;
use crew_hive::StubPlanner;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// `run_with` is the injectable core these tests drive directly (bypassing
/// `run_task`'s provider discovery), but it still calls
/// `specialists::record` on the bare, CWD-based path (`base_dir()` falls
/// back to `Path::new(".")` with no `CREW_PROJECT_DIR` set) and reads the
/// roster via `Registry::discover()` — both of which, unguarded, land a real
/// `./.crew/specialists.json` in the crate's own working tree under `cargo
/// test`. `testenv::mock` isolates `CREW_PROJECT_DIR` (and
/// `CREW_BROKER_MOCK_REPLY`, unused here since planning/execution are
/// injected explicitly) the same way every other test file that reaches the
/// specialist store does.
fn collect(task: &str, cancel: Arc<AtomicBool>) -> Vec<PluginEvent> {
    let _env = testenv::mock("unused");
    collect_with_model(task, "", cancel)
}

/// [`collect`] minus the env guard — for tests that hold their own
/// `testenv::mock` so they can override `CREW_PROJECT_DIR` under its lock.
fn collect_with_model(task: &str, model: &str, cancel: Arc<AtomicBool>) -> Vec<PluginEvent> {
    let mut evs = Vec::new();
    run_with(
        task,
        Arc::new(StubPlanner { fanout: 2 }),
        Arc::new(StubFactory),
        None,
        model,
        cancel,
        None,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

/// The last `Roster` the run emitted (the post-planning re-emit).
fn last_roster(evs: &[PluginEvent]) -> Option<&[crate::AgentInfo]> {
    evs.iter().rev().find_map(|e| match e {
        PluginEvent::Roster { agents } => Some(agents.as_slice()),
        _ => None,
    })
}

#[test]
fn plain_task_emits_plan_then_hive_events_then_no_summary() {
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
    // A clean run closes silently — no "swarm done" chrome. The sink tasks'
    // answers already streamed as their own per-task messages.
    assert!(
        !evs.iter()
            .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("swarm done"))),
        "a clean run must not emit a swarm-done summary: {evs:?}"
    );
}

// The merge (sink) task's output streams live as its own per-task Message the
// moment it completes (OutputChunk -> translate). A clean run adds no closing
// summary at all, so that answer appears exactly once.
#[test]
fn a_clean_run_streams_sink_output_once_with_no_summary() {
    let evs = collect("build the thing", Arc::new(AtomicBool::new(false)));
    // The merge task (id 2, depending on both leaves) streamed its own output.
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("deps=2"))),
        "expected the merge task's own streamed output message: {evs:?}"
    );
    // No status-only summary follows to duplicate or clutter it.
    assert!(
        !evs.iter().any(|e| matches!(
            e,
            PluginEvent::Message { text, .. }
                if text.contains("swarm done") || text.contains("swarm finished")
        )),
        "a clean run must not emit any swarm summary: {evs:?}"
    );
}

// The footer's model segment reads the roster; that roster must name the
// run's invented cast with the model actually serving it — built from
// memory, not re-read from disk, or it goes empty exactly when the broker
// can't persist specialists (see the test after this one).
#[test]
fn swarm_roster_names_the_cast_with_the_serving_model() {
    let _env = testenv::mock("unused");
    let evs = collect_with_model(
        "build the thing",
        "qwen-test",
        Arc::new(AtomicBool::new(false)),
    );
    let roster = last_roster(&evs).expect("a roster follows the plan");
    let names: Vec<&str> = roster.iter().map(|a| a.name.as_str()).collect();
    assert!(
        names.contains(&"leaf-0") && names.contains(&"merge"),
        "cast missing from roster: {names:?}"
    );
    assert!(
        roster.iter().all(|a| a.model == "qwen-test"),
        "cast must carry the serving model: {roster:?}"
    );
}

// A broker launched from Finder/Dock runs at `/`: `.crew/` is not creatable
// there, `specialists::record` fails silently, and a roster re-read from
// disk comes back empty — which is how the footer lost its model segment in
// the field (v0.10.1). The cast must survive in memory.
#[test]
fn swarm_roster_survives_an_unwritable_project_dir() {
    let _env = testenv::mock("unused");
    std::env::set_var("CREW_PROJECT_DIR", "/dev/null/nowhere");
    let evs = collect_with_model(
        "build the thing",
        "qwen-test",
        Arc::new(AtomicBool::new(false)),
    );
    let roster = last_roster(&evs).expect("a roster follows the plan");
    assert!(
        !roster.is_empty(),
        "the cast must not depend on a writable project dir"
    );
    assert!(
        roster.iter().all(|a| a.model == "qwen-test"),
        "cast must carry the serving model: {roster:?}"
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
    let _env = testenv::mock("unused"); // see `collect`'s doc — isolates CREW_PROJECT_DIR
    let mut fail_tasks = std::collections::HashSet::new();
    fail_tasks.insert(TaskId(0));
    let mut evs = Vec::new();
    run_with(
        "build the thing",
        Arc::new(StubPlanner { fanout: 2 }),
        Arc::new(FailingFactory { fail_tasks }),
        None,
        "",
        Arc::new(AtomicBool::new(false)),
        None,
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

    let _env = testenv::mock("unused"); // see `collect`'s doc — isolates CREW_PROJECT_DIR
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
        "",
        Arc::new(AtomicBool::new(false)),
        None,
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

// FINDING 1: `OutputDelta` is published once per SSE fragment by `ApiAgent`,
// so forwarding it raw as `PluginEvent::Hive` would flood the wire with
// exactly what the `TextGate` in `tick.rs` exists to coalesce — and the app
// ignores raw `Hive{OutputDelta}` entirely (`chatswarm.rs`'s no-op arm), so
// every one of those lines is pure waste. Only the coalesced `Delta` from
// `translate` may cross the wire; every other Hive variant must still
// forward raw, unaffected.
#[test]
fn raw_output_delta_never_crosses_the_wire_as_a_hive_event() {
    use crew_hive::agent::{Agent, AgentContext};
    use crew_hive::board::TaskResult;

    struct ChattyAgent;
    impl Agent for ChattyAgent {
        fn run(
            &self,
            ctx: AgentContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = TaskResult> + Send>> {
            Box::pin(async move {
                for frag in ["frag-a", "frag-b", "frag-c"] {
                    ctx.bus.publish(crew_hive::HiveEvent::OutputDelta {
                        agent: ctx.agent.clone(),
                        text: frag.into(),
                    });
                }
                let output = format!("chatty:{}", ctx.task.id.0);
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
    struct ChattyFactory;
    impl crew_hive::AgentFactory for ChattyFactory {
        fn make(&self, _kind: &crew_hive::AgentKind) -> Box<dyn Agent> {
            Box::new(ChattyAgent)
        }
    }

    let _env = testenv::mock("unused"); // see `collect`'s doc — isolates CREW_PROJECT_DIR
    let mut evs = Vec::new();
    run_with(
        "build the thing",
        Arc::new(StubPlanner { fanout: 2 }),
        Arc::new(ChattyFactory),
        None,
        "",
        Arc::new(AtomicBool::new(false)),
        None,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();

    assert!(
        !evs.iter().any(|e| matches!(
            e,
            PluginEvent::Hive {
                event: HiveEvent::OutputDelta { .. }
            }
        )),
        "a raw Hive{{OutputDelta}} crossed the wire, defeating the TextGate: {evs:?}"
    );
    // Other variants must still forward raw, unaffected by the exclusion.
    assert!(
        evs.iter().any(|e| matches!(
            e,
            PluginEvent::Hive {
                event: HiveEvent::AgentSpawned { .. }
            }
        )),
        "AgentSpawned must still forward raw: {evs:?}"
    );
    assert!(
        evs.iter().any(|e| matches!(
            e,
            PluginEvent::Hive {
                event: HiveEvent::TaskStateChanged { .. }
            }
        )),
        "TaskStateChanged must still forward raw: {evs:?}"
    );
    // The coalesced Delta must still reach the wire.
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::Delta { text, .. } if text.contains("frag-a"))),
        "the coalesced Delta must still cross the wire: {evs:?}"
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
            tok_in,
            tok_out,
            cost_microusd,
            ..
        } => Some((
            *exchanges,
            *tokens,
            agent.clone(),
            *tok_in,
            *tok_out,
            *cost_microusd,
        )),
        _ => None,
    });
    let (exchanges, tokens, agent, tok_in, tok_out, cost_microusd) =
        stats.expect("expected an aggregate Stats event");
    assert!(
        tokens > 0,
        "stub agents emit TokenDelta so the aggregate should be > 0"
    );
    assert_eq!(exchanges, 3, "3 stub tasks complete (2 leaves + merge)");
    assert!(
        agent.is_empty(),
        "empty agent = turn-total per protocol.rs Stats docs"
    );
    assert!(tok_in > 0, "stub agents emit input tokens in TokenDelta");
    assert!(tok_out > 0, "stub agents emit output tokens in TokenDelta");
    assert_eq!(
        cost_microusd, 0,
        "stub agents emit no CostDelta, so the aggregate must not invent cost"
    );
    assert_eq!(
        tok_in + tok_out,
        tokens,
        "tok_in + tok_out should sum to the total tokens"
    );
    // The aggregate Stats lands near the end of the run (after the per-task
    // telemetry), even though a clean run emits no summary message after it.
    let stats_pos = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::Stats { .. }))
        .unwrap();
    let last_hive = evs
        .iter()
        .rposition(|e| matches!(e, PluginEvent::Hive { .. }))
        .unwrap();
    assert!(
        stats_pos > last_hive,
        "the aggregate Stats must land after the per-task Hive telemetry"
    );
}

// The roster matches active agents by name, so Activity must carry the
// specialty — with the title, a roster entry could never light up. `translate` is no longer handed the titles at all, so the
// old bug is now impossible rather than merely tested against; this pins the
// name it DOES use.
#[test]
fn activity_names_the_specialist_not_the_task_title() {
    let mut specialties = HashMap::new();
    specialties.insert(TaskId(0), "archivist".to_string());
    let mut agent_task = HashMap::new();

    let evs = translate(
        &HiveEvent::AgentSpawned {
            agent: AgentId(1),
            task: TaskId(0),
        },
        &specialties,
        &mut agent_task,
        &mut HashMap::new(),
        0,
    );
    match &evs[0] {
        PluginEvent::Activity { agent, .. } => assert_eq!(agent, "archivist"),
        other => panic!("expected Activity, got {other:?}"),
    }
}

#[test]
fn lagged_note_wording_and_plural() {
    assert_eq!(
        super::lagged_note(1),
        "telemetry gap: 1 event dropped (bus overflow) \u{2014} task stats may under-count"
    );
    assert!(super::lagged_note(42).contains("42 events dropped"));
}

#[test]
fn output_delta_coalesces_per_agent_and_never_crosses_agents() {
    let mut specialties = HashMap::new();
    specialties.insert(TaskId(1), "planner".to_string());
    specialties.insert(TaskId(2), "coder".to_string());
    let mut agent_task = HashMap::new();
    let mut gates: HashMap<u64, TextGate> = HashMap::new();

    // Spawns teach `agent_task` which task (and so which specialty) each
    // AgentId belongs to — delta naming depends on it.
    for (a, t) in [(10u64, TaskId(1)), (20, TaskId(2))] {
        translate(
            &HiveEvent::AgentSpawned {
                agent: AgentId(a),
                task: t,
            },
            &specialties,
            &mut agent_task,
            &mut gates,
            0,
        );
    }

    let d = |a: u64, t: &str| HiveEvent::OutputDelta {
        agent: AgentId(a),
        text: t.into(),
    };
    let one = |evs: &[PluginEvent]| match evs {
        [PluginEvent::Delta { agent, text }] => (agent.clone(), text.clone()),
        other => panic!("expected exactly one Delta, got {other:?}"),
    };

    // First fragment per agent flushes immediately.
    let a0 = translate(
        &d(10, "plan-"),
        &specialties,
        &mut agent_task,
        &mut gates,
        0,
    );
    let b0 = translate(
        &d(20, "code-"),
        &specialties,
        &mut agent_task,
        &mut gates,
        0,
    );
    // Inside the 80ms gap: buffered, nothing emitted.
    let a1 = translate(
        &d(10, "more"),
        &specialties,
        &mut agent_task,
        &mut gates,
        10,
    );
    // Past the gap: one Delta carrying the buffered text plus this fragment.
    let a2 = translate(&d(10, "!"), &specialties, &mut agent_task, &mut gates, 200);

    assert_eq!(one(&a0), ("planner".to_string(), "plan-".to_string()));
    assert_eq!(one(&b0), ("coder".to_string(), "code-".to_string()));
    assert!(
        a1.is_empty(),
        "a fragment inside the gap buffers, not emits"
    );
    assert_eq!(
        one(&a2),
        ("planner".to_string(), "more!".to_string()),
        "buffered text flushes with the next fragment, and coder's text never leaks in"
    );
}

// Specialty is LLM-authored free text with no uniqueness guarantee, and the
// scheduler runs up to CONCURRENCY tasks in parallel, so two concurrently
// running tasks CAN share a specialty. If `gates` were keyed by that name
// (as it once was), both AgentIds would collapse onto one TextGate and their
// fragments would interleave into a single Delta — genuine cross-agent text
// concatenation. Keying by AgentId (`agent.0`) instead must keep every
// agent's buffer fully separate regardless of naming collisions.
#[test]
fn output_delta_gates_stay_separate_even_when_two_agents_share_a_specialty() {
    let mut specialties = HashMap::new();
    specialties.insert(TaskId(1), "coder".to_string());
    specialties.insert(TaskId(2), "coder".to_string()); // same name, different task
    let mut agent_task = HashMap::new();
    let mut gates: HashMap<u64, TextGate> = HashMap::new();

    for (a, t) in [(10u64, TaskId(1)), (20, TaskId(2))] {
        translate(
            &HiveEvent::AgentSpawned {
                agent: AgentId(a),
                task: t,
            },
            &specialties,
            &mut agent_task,
            &mut gates,
            0,
        );
    }

    let d = |a: u64, t: &str| HiveEvent::OutputDelta {
        agent: AgentId(a),
        text: t.into(),
    };
    let one = |evs: &[PluginEvent]| match evs {
        [PluginEvent::Delta { agent, text }] => (agent.clone(), text.clone()),
        other => panic!("expected exactly one Delta, got {other:?}"),
    };

    // Both agents' first fragment, at the SAME clock tick, must EACH flush
    // immediately — a shared gate would have buffered the second as if it
    // were the first agent's continuation.
    let a0 = translate(
        &d(10, "alpha-"),
        &specialties,
        &mut agent_task,
        &mut gates,
        0,
    );
    let b0 = translate(
        &d(20, "beta-"),
        &specialties,
        &mut agent_task,
        &mut gates,
        0,
    );
    assert_eq!(one(&a0), ("coder".to_string(), "alpha-".to_string()));
    assert_eq!(one(&b0), ("coder".to_string(), "beta-".to_string()));

    // Both, inside their own 80ms gap: both buffer, into separate buffers.
    let a1 = translate(
        &d(10, "-omega"),
        &specialties,
        &mut agent_task,
        &mut gates,
        10,
    );
    let b1 = translate(
        &d(20, "-zed"),
        &specialties,
        &mut agent_task,
        &mut gates,
        10,
    );
    assert!(a1.is_empty(), "agent 10 buffers inside the gap");
    assert!(b1.is_empty(), "agent 20 buffers inside its own gap");

    // Past the gap: each flush carries ONLY its own buffered text, never
    // the other agent's, even though both are named "coder".
    let a2 = translate(&d(10, "!"), &specialties, &mut agent_task, &mut gates, 200);
    let b2 = translate(&d(20, "!"), &specialties, &mut agent_task, &mut gates, 200);
    assert_eq!(
        one(&a2),
        ("coder".to_string(), "-omega!".to_string()),
        "agent 10's flush must never contain agent 20's buffered text"
    );
    assert_eq!(
        one(&b2),
        ("coder".to_string(), "-zed!".to_string()),
        "agent 20's flush must never contain agent 10's buffered text"
    );
}

// ---------------------------------------------------------------------------
// Tools reach the swarm
// ---------------------------------------------------------------------------

/// A provider that asks for a tool the first time it sees a task and answers
/// once the result comes back. Keyed off the prompt rather than a call
/// counter, so it stays deterministic with `CONCURRENCY` agents interleaving.
struct AskThenAnswer;

impl crew_hive::Provider for AskThenAnswer {
    fn complete(
        &self,
        req: crew_hive::CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<crew_hive::Completion, crew_hive::ProviderError>,
                > + Send,
        >,
    > {
        let answered = req.prompt.contains("TOOL EXCHANGES SO FAR");
        Box::pin(async move {
            Ok(crew_hive::Completion {
                text: if answered {
                    "the forecast is 4C".to_string()
                } else {
                    "@tool weather:current {\"q\":\"Oslo\"}".to_string()
                },
                input_tokens: 1,
                output_tokens: 1,
                cost_microusd: 0,
                ..Default::default()
            })
        })
    }
}

struct CountingTools(Arc<std::sync::Mutex<Vec<String>>>);

impl crew_hive::Tools for CountingTools {
    fn hint(&self) -> String {
        "TOOLS: @tool weather:current".into()
    }
    fn call(&self, server: &str, tool: &str, _args: &str) -> Result<String, String> {
        self.0.lock().unwrap().push(format!("{server}:{tool}"));
        Ok("Oslo 4C clear".into())
    }
}

#[test]
fn a_swarm_agent_calls_a_tool_and_the_transcript_shows_it() {
    let _env = testenv::mock("unused");
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let factory = Arc::new(
        crew_hive::ApiFactory::new(Arc::new(AskThenAnswer), 256)
            .with_tools(Arc::new(CountingTools(Arc::clone(&calls)))),
    );

    let mut evs = Vec::new();
    run_with(
        "what is the weather",
        Arc::new(StubPlanner { fanout: 2 }),
        factory,
        None,
        "test-model",
        Arc::new(AtomicBool::new(false)),
        None,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();

    // Every task in the plan reached the tool — this is the whole point of
    // Pillar 1: the PARALLEL engine can now touch the world.
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 3, "one call per planned task: {calls:?}");
    assert!(calls.iter().all(|c| c == "weather:current"));

    // And it is visible: the pane shows the call, not just the answer.
    let texts: Vec<&str> = evs
        .iter()
        .filter_map(|e| match e {
            PluginEvent::Message { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("[tool] weather:current")),
        "no tool line in transcript: {texts:?}"
    );
    // A successful result is not echoed as its own message; the agent's answer
    // is what the reader gets.
    assert!(texts.iter().any(|t| t.contains("the forecast is 4C")));
}

/// The mock provider must never execute a tool.
///
/// `CREW_BROKER_MOCK_REPLY` is a FIXED string returned to every agent, and the
/// GUI screenshot harness sets it. If that string ever ends in something the
/// parser reads as `@tool sys:run …` — a reply about tools, a pasted example,
/// a doc snippet — attaching tools here would turn a screenshot test into a
/// shell command. `swarmconf::backend` withholds them on this arm for that
/// reason; this pins it, because the withholding is invisible in the type.
#[test]
fn the_mock_arm_gets_no_tools_even_when_the_session_has_them() {
    let _env = testenv::mock("@tool weather:current {}");
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let tools: Arc<dyn crew_hive::Tools> = Arc::new(CountingTools(Arc::clone(&calls)));

    let (_planner, factory, _budget, model, _replan) = super::swarmconf::backend(Some(tools));
    assert_eq!(
        model, "mock",
        "this test is only meaningful on the mock arm"
    );

    // Run one agent from that factory: its reply IS a tool directive.
    let bus = crew_hive::EventBus::new(32);
    let agent = factory.make(&crew_hive::AgentKind::Api { system: None });
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let out = rt.block_on(agent.run(crew_hive::AgentContext {
        budget: crew_hive::ToolBudget::solo(),
        agent: crew_hive::AgentId(0),
        task: crew_hive::TaskSpec {
            id: crew_hive::TaskId(0),
            title: "t".into(),
            agent: crew_hive::AgentKind::Api { system: None },
            model: crew_hive::ModelTier::Cheap,
            deps: vec![],
            prompt: "anything".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus,
    }));

    assert!(calls.lock().unwrap().is_empty(), "the mock arm ran a tool");
    // The directive comes back as plain text, which is all it is here.
    assert_eq!(out.output, "@tool weather:current {}");
}

/// The native path, end to end through `run_with`: a tool-speaking provider
/// plus the session's real tool surface. The text convention never appears.
struct NativeProvider;

impl crew_hive::Provider for NativeProvider {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(
        &self,
        req: crew_hive::CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<crew_hive::Completion, crew_hive::ProviderError>,
                > + Send,
        >,
    > {
        // Answer once results have come back; call a tool the first time.
        let answered = req
            .turns
            .iter()
            .any(|t| matches!(t, crew_hive::provider::Turn::ToolResults(_)));
        // The tools it was offered must include the built-in surface.
        let has_run = req.tools.iter().any(|t| t.name == "sys__run");
        Box::pin(async move {
            Ok(crew_hive::Completion {
                text: if answered {
                    "listed it".into()
                } else {
                    String::new()
                },
                input_tokens: 1,
                output_tokens: 1,
                cost_microusd: 0,
                calls: if answered || !has_run {
                    vec![]
                } else {
                    vec![crew_hive::ToolInvocation {
                        id: "c1".into(),
                        name: "sys__run".into(),
                        input: serde_json::json!({"cmd": "echo native-path-works"}),
                    }]
                },
            })
        })
    }
}

#[test]
fn a_swarm_agent_runs_a_real_sys_command_over_native_tool_use() {
    // `testenv::mock` would disable the sys surface (it sets
    // CREW_BROKER_MOCK_REPLY), so the session's tools are built explicitly
    // with the surface ON — the same object `Session::tools` hands the swarm.
    let _env = testenv::mock("unused");
    let session = Session::new();
    let tools = session
        .tools_with_sys(true)
        .expect("a session with sys tools has a surface");

    let factory =
        Arc::new(crew_hive::ApiFactory::new(Arc::new(NativeProvider), 256).with_tools(tools));

    let mut evs = Vec::new();
    run_with(
        "list the directory",
        Arc::new(StubPlanner { fanout: 1 }),
        factory,
        None,
        "test-model",
        Arc::new(AtomicBool::new(false)),
        None,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();

    let texts: Vec<&str> = evs
        .iter()
        .filter_map(|e| match e {
            PluginEvent::Message { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    // A REAL shell command ran, through the gate, from a parallel swarm agent.
    assert!(
        texts.iter().any(|t| t.starts_with("[tool] sys:run")),
        "no sys:run in transcript: {texts:?}"
    );
    assert!(texts.iter().any(|t| t.contains("listed it")), "{texts:?}");
}

// ---------------------------------------------------------------------------
// Tool results are shown, not dropped
// ---------------------------------------------------------------------------

fn tool_result_card(ok: bool, ms: u64, text: &str) -> String {
    let mut specialties = HashMap::new();
    specialties.insert(TaskId(0), "api-consumer".to_string());
    let mut agent_task = HashMap::new();
    agent_task.insert(1u64, TaskId(0));
    let evs = translate(
        &HiveEvent::ToolResult {
            agent: AgentId(1),
            label: "sys:run".into(),
            ok,
            text: text.into(),
            ms,
        },
        &specialties,
        &mut agent_task,
        &mut HashMap::new(),
        0,
    );
    match evs.into_iter().next() {
        Some(PluginEvent::Message { sender, text, .. }) => {
            assert_eq!(sender, "api-consumer", "the caller is named, not the tool");
            text
        }
        other => panic!("expected a Message, got {other:?}"),
    }
}

/// A successful result used to produce NOTHING, so what an API actually
/// returned was unreachable — only the agent's paraphrase of it.
#[test]
fn a_successful_result_reaches_the_transcript() {
    let card = tool_result_card(true, 1_200, "Oslo: +56F\nTokyo: +78F");
    let mut lines = card.lines();
    // The first line is the whole card when folded, so it has to carry the
    // outcome and the duration.
    assert_eq!(lines.next().unwrap(), "[tool] sys:run \u{2713} 1s");
    // …and the output is under it, for the click.
    assert_eq!(lines.next().unwrap(), "Oslo: +56F");
}

#[test]
fn a_failure_keeps_its_mark_and_its_text() {
    let card = tool_result_card(false, 300, "connection refused");
    assert!(card.starts_with("[tool] sys:run \u{2717} 0.3s"), "{card}");
    assert!(card.contains("connection refused"), "{card}");
}

/// A tool that returns nothing must not leave a card with a trailing blank
/// body — one line in, one line out.
#[test]
fn an_empty_result_is_a_single_line() {
    let card = tool_result_card(true, 40, "   \n\n");
    assert_eq!(card, "[tool] sys:run \u{2713} 0.0s");
}

/// The transcript is held in memory and a `curl` of a large page has no
/// ceiling of its own.
#[test]
fn a_huge_result_is_bounded() {
    let card = tool_result_card(true, 100, &"x".repeat(super::swarmmsg::RESULT_CLIP * 3));
    assert!(
        card.chars().count() < super::swarmmsg::RESULT_CLIP + 200,
        "{}",
        card.len()
    );
}
