use super::*;
use crate::broker::testenv;
use crate::PluginEvent;

fn run_loop(rest: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut evs = Vec::new();
    loop_cmd(&mut session, rest, &mut |ev| {
        evs.push(ev);
        Ok(())
    })
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
    let _g = testenv::mock("refined answer\n@done");
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
    let _g = testenv::mock("ok\n@done");
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

#[test]
fn round_body_feeds_the_previous_answer_forward() {
    assert_eq!(round_body("task", None), "task");
    let b = round_body("task", Some("draft v1"));
    assert!(b.starts_with("task"), "{b}");
    assert!(b.contains("draft v1"), "{b}");
    assert!(b.contains("Improve on it"), "{b}");
}
