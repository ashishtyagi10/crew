//! The model sizes the work: `ROUNDS:` and `AGENTS:` parse conservatively,
//! reach the loop/goal/fan arms, and leave the constants as backstops.
use super::*;
use crate::broker::roundloop::MAX_ROUNDS;

fn trio() -> Vec<String> {
    testenv::TRIO.iter().map(|(n, _)| n.to_string()).collect()
}

/// Senders of every `Message` event — the fan names each replier `<name> → user`.
fn senders(evs: &[PluginEvent]) -> Vec<String> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Message { sender, .. } => Some(sender.clone()),
            _ => None,
        })
        .collect()
}

// ── grammar ────────────────────────────────────────────────────────────────

#[test]
fn a_rounds_line_rides_along_with_a_loop() {
    let d = parse_decision("SHAPE: loop\nROUNDS: 5").unwrap();
    assert_eq!(d.shape, Shape::Loop);
    assert_eq!(d.hints.rounds, Some(5));
}

#[test]
fn rounds_after_a_why_line_still_parse() {
    let d = parse_decision("SHAPE: goal\nWHY: needs a judge\nROUNDS: 4.").unwrap();
    assert_eq!(d.why.as_deref(), Some("needs a judge"));
    assert_eq!(d.hints.rounds, Some(4));
}

#[test]
fn rounds_are_clamped_into_the_loops_legal_range() {
    let big = parse_decision("SHAPE: loop\nROUNDS: 99").unwrap();
    assert_eq!(big.hints.rounds, Some(MAX_ROUNDS));
    let zero = parse_decision("SHAPE: loop\nROUNDS: 0").unwrap();
    assert_eq!(zero.hints.rounds, Some(1), "asked for work, not silence");
}

#[test]
fn junk_rounds_drop_the_hint_and_keep_the_shape() {
    for reply in [
        "SHAPE: loop\nROUNDS: many",
        "SHAPE: loop\nROUNDS:",
        "SHAPE: loop\nROUNDS: -3",
    ] {
        let d = parse_decision(reply).unwrap();
        assert_eq!(d.shape, Shape::Loop, "{reply:?}");
        assert_eq!(d.hints.rounds, None, "{reply:?}");
    }
}

#[test]
fn an_agents_line_is_filtered_to_the_roster_in_roster_spelling() {
    let d = parse_decision_on("SHAPE: fan\nAGENTS: Coder, mallory, REVIEWER", &trio()).unwrap();
    assert_eq!(
        d.hints.agents,
        Some(vec!["coder".to_string(), "reviewer".to_string()])
    );
}

#[test]
fn agents_split_on_plus_like_the_hand_typed_subset() {
    let d = parse_decision_on("SHAPE: fan\nAGENTS: planner+coder", &trio()).unwrap();
    assert_eq!(
        d.hints.agents,
        Some(vec!["planner".to_string(), "coder".to_string()])
    );
}

#[test]
fn unknown_only_agents_fall_back_to_everyone() {
    let d = parse_decision_on("SHAPE: fan\nAGENTS: mallory, trent", &trio()).unwrap();
    assert_eq!(d.hints.agents, None);
    // Not even a near miss: exact names only, never fuzzy.
    let d = parse_decision_on("SHAPE: fan\nAGENTS: coders", &trio()).unwrap();
    assert_eq!(d.hints.agents, None);
}

#[test]
fn with_no_roster_an_agents_line_names_nobody() {
    let d = parse_decision("SHAPE: fan\nAGENTS: coder").unwrap();
    assert_eq!(d.hints.agents, None);
}

#[test]
fn a_hint_the_shape_cannot_use_is_dropped() {
    let d = parse_decision_on("SHAPE: fan\nROUNDS: 5", &trio()).unwrap();
    assert_eq!(d.hints.rounds, None, "a fan has no rounds");
    let d = parse_decision_on("SHAPE: loop\nAGENTS: coder", &trio()).unwrap();
    assert_eq!(d.hints.agents, None, "a loop has no subset");
}

#[test]
fn neither_sizing_line_can_change_the_shape() {
    let d = parse_decision_on("SHAPE: reply\nROUNDS: 5\nAGENTS: coder", &trio()).unwrap();
    assert_eq!(d.shape, Shape::Reply);
    assert_eq!(d.hints, Hints::default());
    assert_eq!(parse_decision_on("ROUNDS: 5\nSHAPE: loop", &trio()), None);
}

// ── dispatch ───────────────────────────────────────────────────────────────

#[test]
fn a_rounds_hint_reaches_the_loop_as_its_count() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: loop\nROUNDS: 5".to_string());
    let evs = route_stubbed("polish the intro", &call);
    assert!(any_text(&evs, "loop round 1/5"), "{evs:?}");
    assert!(any_text(&evs, "5 round(s) complete"), "{evs:?}");
}

#[test]
fn no_rounds_hint_means_the_loop_backstop() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: loop\nWHY: iterate".to_string());
    let evs = route_stubbed("polish the intro", &call);
    assert!(any_text(&evs, "loop round 1/3"), "{evs:?}");
    assert!(any_text(&evs, "3 round(s) complete"), "{evs:?}");
}

#[test]
fn an_over_large_rounds_hint_stops_at_the_ceiling() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: loop\nROUNDS: 99".to_string());
    let evs = route_stubbed("polish the intro", &call);
    assert!(
        any_text(&evs, &format!("loop round 1/{MAX_ROUNDS}")),
        "{evs:?}"
    );
    assert!(
        any_text(&evs, &format!("{MAX_ROUNDS} round(s) complete")),
        "{evs:?}"
    );
    assert!(!any_text(&evs, "round 11/"), "{evs:?}");
}

#[test]
fn a_rounds_hint_reaches_the_goal_as_its_cap() {
    // The judge never rules MET, so the cap is what ends the run.
    let _g = testenv::mock_with_specialists("NOT MET: still rough\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: goal\nROUNDS: 2".to_string());
    let evs = route_stubbed("keep working until it round-trips", &call);
    assert!(any_text(&evs, "goal round 1/2"), "{evs:?}");
    assert!(any_text(&evs, "goal round 2/2"), "{evs:?}");
    assert!(any_text(&evs, "goal not met after 2 rounds"), "{evs:?}");
}

#[test]
fn an_agents_hint_fans_to_that_subset_only() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: fan\nAGENTS: coder".to_string());
    let evs = route_stubbed("compare approaches", &call);
    assert!(
        any_text(&evs, "fanning out to coder in parallel"),
        "{evs:?}"
    );
    let who = senders(&evs);
    assert!(who.contains(&"coder \u{2192} user".to_string()), "{who:?}");
    assert!(
        !who.contains(&"planner \u{2192} user".to_string()),
        "{who:?}"
    );
    assert!(
        !who.contains(&"reviewer \u{2192} user".to_string()),
        "{who:?}"
    );
    assert!(
        any_text(&evs, "fan done \u{2014} 1 of 1 replied"),
        "{evs:?}"
    );
}

#[test]
fn an_unknown_only_agents_hint_fans_to_everyone() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = |_: &str| Ok("SHAPE: fan\nAGENTS: mallory".to_string());
    let evs = route_stubbed("compare approaches", &call);
    assert!(any_text(&evs, "fanning out to 3 agents"), "{evs:?}");
    assert!(
        any_text(&evs, "fan done \u{2014} 3 of 3 replied"),
        "{evs:?}"
    );
}
