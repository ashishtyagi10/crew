//! A run where some tasks failed still ends in the lead's answer, whose brief
//! names what failed, and is never judged; a run where nothing finished keeps
//! the status line alone. Stub workers; a failing one says "stub failure".
use super::super::swarmanswer::SynthFn;
use super::super::swarmverify::{Judge, Verify};
use super::super::SWARM_LEAD;
use super::*;
use crate::broker::testenv;
use crate::protocol::PluginEvent;
use crew_hive::agent::FailingFactory;
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

/// A plan of `(title, deps)`, ids in order.
fn plan(tasks: &[(&str, &[u64])]) -> Vec<TaskSpec> {
    let spec = |id: usize, &(title, deps): &(&str, &[u64])| TaskSpec {
        id: TaskId(id as u64),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: title.into(),
        specialty: format!("{title}::x"),
        expertise: String::new(),
    };
    tasks.iter().enumerate().map(|(i, t)| spec(i, t)).collect()
}

/// Three independent tasks; the third is the one the tests fail.
pub(super) fn three() -> Vec<TaskSpec> {
    plan(&[
        ("Read the code", &[]),
        ("Write the fix", &[]),
        ("Run tests", &[]),
    ])
}

/// A call (closing answer or judge) that keeps every brief and answers `reply`.
fn recording(reply: &'static str) -> (Arc<Mutex<Vec<String>>>, Box<SynthFn>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    let call = move |p: &str| {
        log.lock().unwrap().push(p.to_string());
        Ok(reply.to_string())
    };
    (seen, Box::new(call))
}

/// Run `specs` with the tasks in `fail` failing: the events, and the turn the
/// run hands the thread.
fn run(
    specs: Vec<TaskSpec>,
    fail: &[u64],
    synth: Option<&SynthFn>,
    verify: Verify<'_>,
) -> (Vec<PluginEvent>, Option<String>) {
    let _env = testenv::mock("unused");
    let fail_tasks = fail.iter().map(|i| TaskId(*i)).collect();
    let mut evs = Vec::new();
    let reply = super::super::run_with_synth(
        "fix the bug",
        Arc::new(Fixed(specs)),
        Arc::new(FailingFactory { fail_tasks }),
        None,
        "",
        Arc::new(AtomicBool::new(false)),
        None,
        synth,
        verify,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    (evs, reply)
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
fn a_partial_run_answers_once_from_what_finished_and_names_what_failed() {
    let answer = "Fixed in lib.rs; the tests could not be run.";
    let (briefs, call) = recording(answer);
    let (evs, reply) = run(three(), &[2], Some(&*call), None);
    let status = "swarm finished with 1 failed task(s)";
    assert_eq!(
        smith_lines(&evs),
        [answer, status],
        "one answer, then the status"
    );
    let briefs = briefs.lock().unwrap();
    assert_eq!(briefs.len(), 1, "exactly one closing call");
    let brief = &briefs[0];
    assert!(
        brief.ends_with(
            "\n\nMISSING:\nThese sub-tasks FAILED and have no result: \"Run tests\" \
             (stub failure). Answer from what finished, and say plainly what could not be done."
        ),
        "the brief names the failed task and why: {brief}"
    );
    assert!(
        brief.contains("## Read the code\nstub:0") && brief.contains("## Write the fix\nstub:1"),
        "the finished outputs: {brief}"
    );
    assert!(!brief.contains("## Run tests"), "not an output: {brief}");
    assert_eq!(reply.as_deref(), Some(answer), "the thread keeps it");
}

#[test]
fn a_partial_run_answers_even_when_what_finished_ends_in_one_sink() {
    // `gather` and `left` finished; `left` alone is a sink, which on a clean
    // run means "its reply is the answer" — but here `right` is missing.
    let diamond = plan(&[("gather", &[]), ("left", &[0]), ("right", &[0])]);
    let (briefs, call) = recording("Left is A; right could not be checked.");
    let (evs, _) = run(diamond, &[2], Some(&*call), None);
    let briefs = briefs.lock().unwrap();
    assert_eq!(briefs.len(), 1, "the gap has to be said: {evs:?}");
    assert!(
        briefs[0].contains("\"right\" (stub failure)"),
        "{}",
        briefs[0]
    );
}

#[test]
fn a_partial_run_is_never_judged_and_never_sent_back() {
    let (_, call) = recording("Fixed; the tests could not be run.");
    let (verdicts, judge) = recording("NOT MET: the tests did not run");
    let (evs, _) = run(three(), &[2], Some(&*call), Some(Judge::new(&*judge)));
    let judged = verdicts.lock().unwrap().len();
    assert_eq!(judged, 0, "no judge call on a partial run");
    let plans = evs
        .iter()
        .filter(|e| matches!(e, PluginEvent::HivePlan { .. }))
        .count();
    assert_eq!(plans, 1, "no revision pass: {evs:?}");
    assert!(!smith_lines(&evs)
        .iter()
        .any(|l| l.starts_with("not yet") || l.starts_with("verified")));
}

#[test]
fn a_run_where_every_task_failed_has_no_answer_only_the_status_line() {
    let (briefs, call) = recording("never");
    let (evs, reply) = run(plan(&[("a", &[]), ("b", &[])]), &[0, 1], Some(&*call), None);
    assert!(briefs.lock().unwrap().is_empty(), "nothing to answer from");
    assert_eq!(smith_lines(&evs), ["swarm finished with 2 failed task(s)"]);
    assert_eq!(reply, None);
}

#[test]
fn a_keyless_partial_run_emits_exactly_the_status_line_it_did_before() {
    let (evs, reply) = run(three(), &[2], None, None);
    assert_eq!(smith_lines(&evs), ["swarm finished with 1 failed task(s)"]);
    assert!(!evs
        .iter()
        .any(|e| matches!(e, PluginEvent::Activity { agent, .. } if agent == SWARM_LEAD)));
    assert_eq!(reply, None, "no answer, so no turn");
}

#[test]
fn a_clean_runs_brief_says_nothing_about_failures() {
    let (briefs, call) = recording("All three done.");
    run(three(), &[], Some(&*call), None);
    let brief = &briefs.lock().unwrap()[0];
    assert!(
        !brief.contains("MISSING") && !brief.contains("FAILED"),
        "{brief}"
    );
}
