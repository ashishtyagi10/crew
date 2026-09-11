//! The model asks for a check: `VERIFY: yes` parses conservatively, survives
//! only on the shapes that can act on it, and is said in the routing line.
use super::*;

#[test]
fn a_verify_line_rides_along_with_a_swarm() {
    let d = parse_decision("SHAPE: swarm\nWHY: tests must pass\nVERIFY: yes").unwrap();
    assert_eq!(d.shape, Shape::Swarm);
    assert!(d.hints.verify);
    assert_eq!(d.why.as_deref(), Some("tests must pass"));
}

#[test]
fn a_verify_line_is_kept_for_a_plan_that_will_become_a_swarm() {
    let d = parse_decision("SHAPE: plan\nVERIFY: yes").unwrap();
    assert!(d.hints.verify);
}

#[test]
fn a_verify_line_is_dropped_for_every_other_shape() {
    for shape in ["reply", "fan", "loop", "goal", "commit", "review"] {
        let d = parse_decision(&format!("SHAPE: {shape}\nVERIFY: yes")).unwrap();
        assert!(!d.hints.verify, "{shape} cannot act on a verdict");
        assert_eq!(d.hints, Hints::default(), "{shape}: nothing else changes");
    }
}

#[test]
fn only_a_plain_yes_asks_for_a_check() {
    for reply in [
        "SHAPE: swarm\nVERIFY: no",
        "SHAPE: swarm\nVERIFY: maybe",
        "SHAPE: swarm\nVERIFY:",
        "SHAPE: swarm\nVERIFY: yes-ish",
        "SHAPE: swarm",
    ] {
        let d = parse_decision(reply).unwrap();
        assert_eq!(d.shape, Shape::Swarm, "{reply:?}");
        assert!(!d.hints.verify, "{reply:?}");
    }
    for reply in ["SHAPE: swarm\nVERIFY: yes.", "SHAPE: swarm\nverify: YES"] {
        assert!(parse_decision(reply).unwrap().hints.verify, "{reply:?}");
    }
}

#[test]
fn the_routing_line_says_verified_after_the_shape() {
    let d = parse_decision("SHAPE: swarm\nWHY: tests must pass\nVERIFY: yes").unwrap();
    assert_eq!(
        Routing::Chosen(d).line(),
        "routing: swarm \u{00b7} verified \u{2014} tests must pass"
    );
    let bare = parse_decision("SHAPE: swarm\nVERIFY: yes").unwrap();
    assert_eq!(
        Routing::Chosen(bare).line(),
        "routing: swarm \u{00b7} verified"
    );
    let plain = parse_decision("SHAPE: swarm\nWHY: two parts").unwrap();
    assert_eq!(
        Routing::Chosen(plain).line(),
        "routing: swarm \u{2014} two parts"
    );
}

#[test]
fn the_classifier_is_told_when_to_ask_for_a_check() {
    let p = classify::prompt("make the tests pass", &World::default());
    assert!(p.contains("`VERIFY: yes`"), "{p}");
    assert!(p.contains("swarm or plan only"), "{p}");
}
