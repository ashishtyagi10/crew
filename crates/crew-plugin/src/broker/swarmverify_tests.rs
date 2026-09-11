//! The judge's seam, driven keyless: a verdict is one line, a miss is one
//! revision through the same planner, and every gate that stops the judge
//! leaves the run exactly as it was.
use super::*;
use crate::broker::testenv;
use crew_hive::agent::StubFactory;
use crew_hive::{AgentKind, ModelTier, PlanError, Planner, TaskSpec};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// A planner that hands back a two-sink graph and keeps every goal it saw.
struct Counting(Arc<Mutex<Vec<String>>>);

impl Planner for Counting {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        self.0.lock().unwrap().push(goal.to_string());
        let spec = |id: u64, title: &str, deps: &[u64]| TaskSpec {
            id: TaskId(id),
            title: title.into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: deps.iter().map(|d| TaskId(*d)).collect(),
            prompt: title.into(),
            specialty: format!("{title}::x"),
            expertise: String::new(),
        };
        let g = TaskGraph::new(vec![
            spec(0, "gather", &[]),
            spec(1, "left", &[0]),
            spec(2, "right", &[0]),
        ])
        .unwrap();
        Box::pin(async move { Ok(g) })
    }
}

/// A judge that records its briefs and answers `reply`.
fn judge(reply: Result<&'static str, &'static str>) -> (Arc<Mutex<Vec<String>>>, Box<SynthFn>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    let call = move |p: &str| {
        log.lock().unwrap().push(p.to_string());
        reply.map(str::to_string).map_err(str::to_string)
    };
    (seen, Box::new(call))
}

/// Drive the core with stub workers, no closing call, and `verify`. Returns
/// the events and every goal the planner was handed.
fn run(cancelled: bool, verify: Verify<'_>) -> (Vec<PluginEvent>, Vec<String>) {
    let _env = testenv::mock("unused");
    let goals = Arc::new(Mutex::new(Vec::new()));
    let mut evs = Vec::new();
    super::super::run_with_synth(
        "make the tests pass",
        Arc::new(Counting(Arc::clone(&goals))),
        Arc::new(StubFactory),
        None,
        "",
        Arc::new(AtomicBool::new(cancelled)),
        None,
        None,
        verify,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    let goals = goals.lock().unwrap().clone();
    (evs, goals)
}

/// The lead's lines, minus the plan line every pass opens with.
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

#[test]
fn a_met_verdict_is_one_verified_line_and_no_second_run() {
    let (briefs, call) = judge(Ok("MET: the suite is green"));
    let (evs, goals) = run(false, Some(Judge::new(&*call)));
    assert_eq!(
        smith_lines(&evs),
        vec!["verified \u{2014} the suite is green".to_string()]
    );
    assert_eq!(goals.len(), 1, "no revision after a met verdict: {goals:?}");
    let brief = &briefs.lock().unwrap()[0];
    assert!(brief.contains("make the tests pass"), "{brief}");
    assert!(
        brief.contains("## left") && brief.contains("## right"),
        "sinks: {brief}"
    );
    assert!(
        !brief.contains("## gather"),
        "a non-sink is not the result: {brief}"
    );
    assert!(brief.contains("`MET: <why>` or `NOT MET:"), "{brief}");
}

#[test]
fn a_bare_met_is_said_as_verified_with_no_dangling_dash() {
    let (_, call) = judge(Ok("MET"));
    let (evs, _) = run(false, Some(Judge::new(&*call)));
    assert_eq!(smith_lines(&evs), vec!["verified".to_string()]);
}

#[test]
fn a_not_met_verdict_says_not_yet_and_runs_exactly_one_revision() {
    let (briefs, call) = judge(Err("boom").or(Ok("NOT MET: two tests still fail")));
    let (evs, goals) = run(false, Some(Judge::new(&*call)));
    let lines = smith_lines(&evs);
    assert_eq!(
        lines,
        vec!["not yet \u{2014} two tests still fail".to_string()]
    );
    assert_eq!(goals.len(), 2, "exactly one revision: {goals:?}");
    let revised = &goals[1];
    assert!(revised.starts_with("REVISE:"), "{revised}");
    assert!(
        revised.contains("a judge found: two tests still fail"),
        "{revised}"
    );
    assert!(
        revised.contains("## left") && revised.contains("stub:1"),
        "prior outputs: {revised}"
    );
    assert!(revised.ends_with("GOAL:\nmake the tests pass"), "{revised}");
    assert_eq!(
        briefs.lock().unwrap().len(),
        1,
        "the second pass is never judged"
    );
    // Two passes, two plans, and the turn settles once, at the very end.
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, PluginEvent::HivePlan { .. }))
            .count(),
        2
    );
    let idles = evs
        .iter()
        .filter(|e| matches!(e, PluginEvent::Activity { agent, state, .. } if agent.is_empty() && state == "idle"))
        .count();
    assert_eq!(idles, 1, "{evs:?}");
    assert!(matches!(evs.last(), Some(PluginEvent::Activity { state, .. }) if state == "idle"));
}

#[test]
fn a_judge_error_is_one_quiet_line_and_the_run_still_reads_clean() {
    let (_, call) = judge(Err("provider fell over"));
    let (evs, goals) = run(false, Some(Judge::new(&*call)));
    assert_eq!(
        smith_lines(&evs),
        vec!["could not verify: provider fell over".to_string()]
    );
    assert_eq!(goals.len(), 1);
    assert!(!evs
        .iter()
        .any(|e| matches!(e, PluginEvent::Message { text, .. } if text.contains("failed"))));
}

#[test]
fn a_cancelled_run_is_never_judged() {
    let (briefs, call) = judge(Ok("MET: fine"));
    let (evs, goals) = run(true, Some(Judge::new(&*call)));
    assert!(
        briefs.lock().unwrap().is_empty(),
        "no judge call on a cancelled run"
    );
    assert_eq!(goals.len(), 1);
    assert!(!smith_lines(&evs).iter().any(|l| l.starts_with("verified")));
}

/// The clock-free shape of a stream: each event's variant, and a message's
/// sender and text. `Stats` carries the run clock, so two runs never match
/// byte for byte; everything the pane shows does.
fn shape(evs: &[PluginEvent]) -> Vec<String> {
    evs.iter()
        .map(|e| match e {
            PluginEvent::Message { sender, text, .. } => format!("Message {sender}: {text}"),
            other => {
                let dbg = format!("{other:?}");
                dbg.split([' ', '{', '(']).next().unwrap().to_string()
            }
        })
        .collect()
}

#[test]
fn with_no_judge_the_stream_is_exactly_what_it_was_before() {
    let (evs, goals) = run(false, None);
    assert!(smith_lines(&evs).is_empty(), "no judge, no line: {evs:?}");
    assert!(!evs
        .iter()
        .any(|e| matches!(e, PluginEvent::Activity { agent, .. } if agent == SWARM_LEAD)));
    assert_eq!(goals.len(), 1);
}

#[test]
fn under_the_mock_provider_a_verify_request_changes_nothing() {
    // The real entry, on routing's gates: the mock provider gets no judge,
    // so the stream is the same whether or not the router asked for one.
    let _env = testenv::mock("unused");
    let session = crate::broker::session::Session::new();
    let mut a = Vec::new();
    super::super::run_task("make the tests pass", false, &session, &mut |ev| {
        a.push(ev);
        Ok(())
    })
    .unwrap();
    let mut b = Vec::new();
    super::super::run_task("make the tests pass", true, &session, &mut |ev| {
        b.push(ev);
        Ok(())
    })
    .unwrap();
    assert_eq!(shape(&a), shape(&b));
    assert!(!smith_lines(&b)
        .iter()
        .any(|l| l.starts_with("verified") || l.starts_with("not yet")));
}

#[test]
fn the_revision_allowance_is_the_named_cap() {
    let (_, call) = judge(Ok("MET"));
    let mut j = Judge::new(&*call);
    let mut passes = 1;
    while let Some(next) = j.next() {
        j = next;
        passes += 1;
    }
    assert_eq!(passes, REVISION_CAP as usize, "judged passes equal the cap");
}
