use super::*;
use crate::broker::testenv;

fn delegated(agent: &'static str) -> Resolved {
    let e = registry::by_name(agent).unwrap();
    Resolved::Delegated {
        name: e.name,
        agent: e.cli.unwrap().bin,
    }
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// A delegated resolution takes the lead even when it is not first on the
/// roster — that is the whole difference from the keyless fallback, which
/// always picks the first registered name.
#[test]
fn a_delegated_resolution_leads_with_its_own_cli() {
    let roster = ["my-agent", "claude", "codex"];
    assert_eq!(
        starter_for(&delegated("claude"), names(&roster)),
        Some("claude".to_string())
    );
    assert_eq!(
        starter_for(&delegated("codex"), names(&roster)),
        Some("codex".to_string())
    );
    // The roster's own capitalisation survives (a manifest may spell it).
    assert_eq!(
        starter_for(&delegated("codex"), names(&["Codex"])),
        Some("Codex".to_string())
    );
}

/// Every non-delegated resolution declines the lead, leaving the keyless
/// fallback (or the intent router) exactly as it was.
#[test]
fn non_delegated_resolutions_decline_the_lead() {
    let roster = names(&["claude", "codex"]);
    for r in [
        Resolved::Mock,
        Resolved::Keyed("dashscope"),
        Resolved::Relay,
        Resolved::None,
    ] {
        assert_eq!(starter_for(&r, roster.clone()), None, "{r:?}");
    }
    // …and a delegated CLI missing from the roster cannot lead either.
    assert_eq!(starter_for(&delegated("codex"), names(&["scout"])), None);
}

/// The swarm's LLM planner needs an API provider; a delegated subscription
/// cannot serve that JSON path, so a delegated-only machine degrades the
/// swarm exactly the way keyless always has (stub planner) — while plain
/// tasks route through the CLI relay instead. `/doctor` states this rather
/// than any path erroring.
#[test]
fn a_delegated_only_machine_degrades_the_swarm_like_keyless() {
    let _g = testenv::no_provider();
    assert!(
        crate::broker::discover::provider_and_model().is_none(),
        "no API provider must resolve for the planner on a keyless machine, \
         whatever the subscription probes say"
    );
}

/// Under the mock guard, live resolution is the mock — proving the harness
/// stays deterministic (no probe can fire: resolve short-circuits first).
#[test]
fn the_mock_guard_still_wins_live_resolution() {
    let _g = testenv::mock("ok");
    assert_eq!(resolved_live(), Resolved::Mock);
}
