use super::*;
use crate::broker::testenv;
use crew_hive::agent::StubFactory;
use crew_hive::{AgentKind, ModelTier, PlanError, Planner, TaskSpec};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// A planner that hands back exactly the graph a test drew.
struct Fixed(Vec<TaskSpec>);

impl Planner for Fixed {
    fn plan(
        &self,
        _goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        let g = TaskGraph::new(self.0.clone()).expect("a valid test graph");
        Box::pin(async move { Ok(g) })
    }
}

fn spec(id: u64, title: &str, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: title.into(),
        specialty: format!("{title}::x"),
        expertise: String::new(),
    }
}

/// Two sinks: `left` and `right` both hang off `gather`, and nothing merges them.
fn diamond() -> Vec<TaskSpec> {
    vec![
        spec(0, "gather", &[]),
        spec(1, "left", &[0]),
        spec(2, "right", &[0]),
    ]
}

/// One sink: `merge` is the answer already.
fn chain() -> Vec<TaskSpec> {
    vec![spec(0, "gather", &[]), spec(1, "merge", &[0])]
}

/// Drive the injectable core with the stub workers and the given closing call.
/// The caller holds `testenv::mock` — see `swarm_tests::collect` for why.
fn run(specs: Vec<TaskSpec>, cancelled: bool, synth: Synth<'_>) -> Vec<PluginEvent> {
    let mut evs = Vec::new();
    super::super::run_with_synth(
        "compare the two",
        Arc::new(Fixed(specs)),
        Arc::new(StubFactory),
        None,
        "",
        Arc::new(AtomicBool::new(cancelled)),
        None,
        synth,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

/// The lead's lines, minus the plan line every run opens with.
fn smith_lines(evs: &[PluginEvent]) -> Vec<String> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Message { sender, text, .. }
                if sender == SWARM_LEAD && !text.starts_with("planned ") =>
            {
                Some(text.clone())
            }
            _ => None,
        })
        .collect()
}

/// A closing call that keeps every brief it was handed and answers `reply`.
fn recording(reply: Result<&'static str, &'static str>) -> (Arc<Mutex<Vec<String>>>, Box<SynthFn>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    let call = move |p: &str| {
        log.lock().unwrap().push(p.to_string());
        reply.map(str::to_string).map_err(str::to_string)
    };
    (seen, Box::new(call))
}

#[test]
fn a_two_sink_run_ends_with_one_answer_from_the_lead_after_the_workers() {
    let _env = testenv::mock("unused");
    let (seen, call) = recording(Ok("Both agree: X."));
    let evs = run(diamond(), false, Some(&*call));

    assert_eq!(smith_lines(&evs), vec!["Both agree: X.".to_string()]);
    let answer = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::Message { text, .. } if text == "Both agree: X."))
        .unwrap();
    let last_worker = evs
        .iter()
        .rposition(|e| matches!(e, PluginEvent::Message { text, .. } if text.starts_with("stub:")))
        .expect("the workers' own replies streamed");
    let stats = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::Stats { .. }))
        .unwrap();
    assert!(
        last_worker < answer,
        "the answer comes after every worker's reply"
    );
    assert!(answer < stats, "and before the run's aggregate Stats");
    let thinking = evs.iter().position(|e| {
        matches!(e, PluginEvent::Activity { agent, state, .. } if agent == SWARM_LEAD && state == "thinking")
    });
    assert!(
        thinking.is_some_and(|t| t < answer),
        "the pane sees the lead working first"
    );

    let briefs = seen.lock().unwrap();
    assert_eq!(briefs.len(), 1, "exactly one closing call");
    let brief = &briefs[0];
    assert!(brief.contains("compare the two"), "the goal: {brief}");
    assert!(
        brief.contains("## left\nstub:1 deps=1"),
        "one sink's output: {brief}"
    );
    assert!(
        brief.contains("## right\nstub:2 deps=1"),
        "the other's: {brief}"
    );
    assert!(
        brief.contains("## gather\nstub:0 deps=0"),
        "and the root's: {brief}"
    );
}

#[test]
fn the_answer_goes_to_the_session_log_like_a_workers_reply() {
    let _env = testenv::mock("unused");
    let (_, call) = recording(Ok("Both agree: X."));
    run(diamond(), false, Some(&*call));
    let dir = std::env::var("CREW_PROJECT_DIR").unwrap();
    let log = std::fs::read_to_string(format!("{dir}/.crew/session-live.md")).unwrap();
    assert!(log.contains("answer: Both agree: X."), "{log}");
}

#[test]
fn a_run_that_already_ends_in_one_merge_task_makes_no_closing_call() {
    let _env = testenv::mock("unused");
    let (seen, call) = recording(Ok("never"));
    let evs = run(chain(), false, Some(&*call));
    assert!(
        seen.lock().unwrap().is_empty(),
        "the merge's reply IS the answer"
    );
    assert!(smith_lines(&evs).is_empty(), "{evs:?}");
}

#[test]
fn a_cancelled_run_makes_no_closing_call() {
    let _env = testenv::mock("unused");
    let (seen, call) = recording(Ok("never"));
    let evs = run(diamond(), true, Some(&*call));
    assert!(seen.lock().unwrap().is_empty());
    assert!(
        smith_lines(&evs)
            .iter()
            .any(|l| l.starts_with("swarm cancelled")),
        "{evs:?}"
    );
}

#[test]
fn a_keyless_run_emits_exactly_what_it_did_before() {
    let _env = testenv::mock("unused");
    let evs = run(diamond(), false, None);
    assert!(smith_lines(&evs).is_empty(), "no call, no line: {evs:?}");
    assert!(!evs
        .iter()
        .any(|e| matches!(e, PluginEvent::Activity { agent, .. } if agent == SWARM_LEAD)));
}

#[test]
fn a_failed_closing_call_is_one_quiet_line_and_the_run_still_reads_clean() {
    let _env = testenv::mock("unused");
    let (_, call) = recording(Err("boom"));
    let evs = run(diamond(), false, Some(&*call));
    assert_eq!(
        smith_lines(&evs),
        vec!["could not combine the workers' answers: boom".to_string()]
    );
    assert!(evs.iter().any(|e| matches!(e, PluginEvent::Stats { .. })));
}

#[test]
fn an_empty_reply_is_said_rather_than_shown_as_a_blank_answer() {
    let _env = testenv::mock("unused");
    let (_, call) = recording(Ok("  \n"));
    let evs = run(diamond(), false, Some(&*call));
    assert_eq!(smith_lines(&evs).len(), 1);
    assert!(smith_lines(&evs)[0].contains("returned nothing"));
}

#[test]
fn one_task_or_one_sink_wants_no_answer_and_two_sinks_do() {
    let one = TaskGraph::new(vec![spec(0, "reply", &[])]).unwrap();
    assert!(!wants_answer(&one, &[TaskId(0)]));
    let merged = TaskGraph::new(chain()).unwrap();
    assert!(!wants_answer(&merged, &[TaskId(0), TaskId(1)]));
    let split = TaskGraph::new(diamond()).unwrap();
    assert!(wants_answer(&split, &[TaskId(0), TaskId(1), TaskId(2)]));
    // Two independent tasks with no root: two sinks, nothing merges them.
    let pair = TaskGraph::new(vec![spec(0, "a", &[]), spec(1, "b", &[])]).unwrap();
    assert!(wants_answer(&pair, &[TaskId(0), TaskId(1)]));
}

#[test]
fn the_brief_holds_every_output_to_the_budget() {
    let one = prompt("goal", &[("a".into(), "x".repeat(10_000))]);
    assert!(
        one.contains("[clipped 6000 chars]"),
        "one output gets OUTPUT_CAP"
    );
    let parts: Vec<(String, String)> = (0..4)
        .map(|i| (format!("t{i}"), "y".repeat(5_000)))
        .collect();
    let four = prompt("goal", &parts);
    assert_eq!(
        four.matches("[clipped 2000 chars]").count(),
        4,
        "four outputs share OUTPUTS_TOTAL_CAP evenly: {}",
        &four[..200]
    );
    let short = prompt("goal", &[("a".into(), "fine".into())]);
    assert!(
        short.ends_with("## a\nfine"),
        "under budget passes through: {short}"
    );
}

#[test]
fn the_closing_line_names_a_cancellation_or_a_failure_and_nothing_on_a_clean_run() {
    let outcome = RunOutcome {
        done: vec![TaskId(0)],
        failed: vec![TaskId(1)],
        cancelled: vec![TaskId(2)],
        tool_rounds: (0, 12),
    };
    assert_eq!(
        closing_line(&outcome, true).as_deref(),
        Some("swarm cancelled (budget or /stop) — 1 done, 1 failed, 1 cancelled")
    );
    assert_eq!(
        closing_line(&outcome, false).as_deref(),
        Some("swarm finished with 1 failed task(s)")
    );
    let clean = RunOutcome {
        done: vec![TaskId(0), TaskId(1)],
        failed: vec![],
        cancelled: vec![],
        tool_rounds: (0, 8),
    };
    assert_eq!(closing_line(&clean, false), None);
}
