use super::*;
use crate::broker::testenv;
use crate::PluginEvent;

fn run_loop(rest: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut evs = Vec::new();
    loop_cmd(
        &mut session,
        rest,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

fn texts(evs: &[PluginEvent]) -> Vec<String> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Message { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn loop_runs_the_requested_rounds_and_reports_done() {
    let _g = testenv::mock_with_specialists("refined answer\n@done", testenv::TRIO);
    let evs = run_loop("3 draft a release plan");
    let ts = texts(&evs);
    let rounds = ts.iter().filter(|t| t.starts_with("loop round")).count();
    assert_eq!(rounds, 3, "{ts:?}");
    assert!(ts.last().unwrap().contains("loop done"), "{ts:?}");
    // Each round actually relayed: three turn summaries.
    assert_eq!(ts.iter().filter(|t| t.starts_with("turn done")).count(), 3);
}

#[test]
fn loop_honours_an_agent_selector() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let evs = run_loop("2 @reviewer critique the design");
    let ts = texts(&evs);
    assert!(
        ts.iter().any(|t| t.contains("starting with reviewer")),
        "{ts:?}"
    );
}

#[test]
fn loop_rejects_bad_counts_and_missing_tasks() {
    let _g = testenv::mock("ok\n@done");
    for bad in ["", "0 task", "99 task", "many task", "3", "3   "] {
        let ts = texts(&run_loop(bad));
        assert_eq!(ts.len(), 1, "{bad:?} → {ts:?}");
        assert!(ts[0].starts_with("usage:"), "{bad:?} → {ts:?}");
    }
}

fn run_goal(rest: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut evs = Vec::new();
    goal_cmd(
        &mut session,
        rest,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

#[test]
fn goal_met_on_round_one_stops_the_loop() {
    // Every mock agent (worker AND judge) replies MET, so round one settles it.
    let _g = testenv::mock_with_specialists("MET: shipped and green\n@done", testenv::TRIO);
    let ts = texts(&run_goal("ship the release"));
    assert!(
        ts.iter().any(|t| t.contains("goal met after 1 round")),
        "{ts:?}"
    );
    assert_eq!(
        ts.iter().filter(|t| t.starts_with("goal round")).count(),
        1,
        "{ts:?}"
    );
}

#[test]
fn goal_gives_up_at_the_round_cap_when_never_met() {
    let _g = testenv::mock_with_specialists("NOT MET: still missing tests\n@done", testenv::TRIO);
    let ts = texts(&run_goal("prove the collatz conjecture"));
    assert_eq!(
        ts.iter().filter(|t| t.starts_with("goal round")).count(),
        GOAL_ROUNDS as usize,
        "{ts:?}"
    );
    assert!(ts.last().unwrap().contains("goal not met after"), "{ts:?}");
    // The judge's reasoning is surfaced each round.
    assert!(
        ts.iter().any(|t| t.contains("still missing tests")),
        "{ts:?}"
    );
}

#[test]
fn goal_without_text_prints_usage() {
    let _g = testenv::mock("ok\n@done");
    let ts = texts(&run_goal("   "));
    assert!(ts[0].starts_with("usage:"), "{ts:?}");
}

#[test]
fn parse_verdict_reads_met_and_not_met() {
    assert_eq!(parse_verdict("MET: all done"), (true, "all done".into()));
    assert_eq!(
        parse_verdict("NOT MET: missing docs"),
        (false, "missing docs".into())
    );
    // Control lines and casing are tolerated; garbage is conservatively not met.
    assert!(parse_verdict("met: fine\n@done").0);
    assert!(!parse_verdict("hard to say").0);
}

fn run_goal_with(rest: &str, elector: crate::broker::intent::Classifier) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut evs = Vec::new();
    goal_cmd_with(
        &mut session,
        rest,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
        Some(elector),
    )
    .unwrap();
    evs
}

/// The judge is the MODEL's choice, not a keyword match on role strings: a
/// stubbed elector names an agent and that agent judges, visibly, in the
/// transcript. `reviewer` is deliberately NOT what the fallback would pick
/// (that would be `coder`, the first non-worker), so a fallback that ignored
/// the elector fails this test.
#[test]
fn a_stubbed_elector_names_the_judge_in_the_transcript() {
    let _g = testenv::mock_with_specialists("MET: shipped\n@done", testenv::TRIO);
    let call = |_: &str| Ok("AGENT: reviewer".to_string());
    let ts = texts(&run_goal_with("ship the release", &call));
    assert!(
        ts.iter()
            .any(|t| t.contains("planner works, reviewer judges")),
        "{ts:?}"
    );
}

/// An off-grammar election reply must not stop the goal: the deterministic
/// fallback (first non-worker) judges instead — which is also exactly what
/// the keyless/mock path gets with no elector at all.
#[test]
fn an_off_grammar_elector_falls_back_to_the_first_non_worker() {
    let _g = testenv::mock_with_specialists("MET: shipped\n@done", testenv::TRIO);
    let call = |_: &str| Ok("probably the reviewer should look".to_string());
    let ts = texts(&run_goal_with("ship the release", &call));
    assert!(
        ts.iter().any(|t| t.contains("planner works, coder judges")),
        "{ts:?}"
    );
}

#[test]
fn a_pre_tripped_stop_flag_cancels_the_loop_before_round_one() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let mut session = Session::new();
    session
        .cancel
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let mut evs = Vec::new();
    loop_cmd(
        &mut session,
        "3 do the thing",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    let ts = texts(&evs);
    assert!(
        ts.iter().any(|t| t.contains("cancelled by /stop")),
        "{ts:?}"
    );
    assert!(
        !ts.iter().any(|t| t.starts_with("loop round")),
        "no rounds ran: {ts:?}"
    );
}

#[test]
fn round_body_feeds_the_previous_answer_forward() {
    assert_eq!(round_body("task", None), "task");
    let b = round_body("task", Some("draft v1"));
    assert!(b.starts_with("task"), "{b}");
    assert!(b.contains("draft v1"), "{b}");
    assert!(b.contains("Improve on it"), "{b}");
}
