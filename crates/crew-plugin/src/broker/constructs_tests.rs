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

#[test]
fn pick_judge_prefers_a_reviewer_who_is_not_the_worker() {
    let names = vec!["planner".to_string(), "coder".into(), "reviewer".into()];
    assert_eq!(pick_judge(&names, "planner"), "reviewer");
    assert_eq!(pick_judge(&names, "reviewer"), "planner");
    assert_eq!(pick_judge(&["solo".to_string()], "solo"), "solo");
}

#[test]
fn pick_judge_keys_off_capability_not_the_literal_reviewer_name() {
    // "opencode" advertises a review capability; even though "coder" comes
    // first, the judge is the critic — so a roster of arbitrarily-named
    // specialists (no agent literally called "reviewer") still elects a judge.
    let names = vec!["planner".to_string(), "coder".into(), "opencode".into()];
    assert_eq!(pick_judge(&names, "planner"), "opencode");
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
