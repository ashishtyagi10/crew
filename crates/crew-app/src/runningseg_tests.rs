//! Footer line 3's running segment and line 1's roster names.
use super::{roster_seg, running_seg};
use crew_plugin::AgentInfo;

fn agent(name: &str) -> AgentInfo {
    AgentInfo {
        name: name.into(),
        role: String::new(),
        model: String::new(),
    }
}

/// The way to stop what is running rides along at every width.
#[test]
fn the_running_segment_always_names_the_way_to_stop() {
    let (what, how) = running_seg(&[1, 2, 3, 4, 5], 120).unwrap();
    assert_eq!(what, "5 running");
    assert_eq!(how.as_deref(), Some("/stop cancels all"));
    let (_, how) = running_seg(&[3, 4], 120).unwrap();
    assert_eq!(how.as_deref(), Some("/stop #3 to cancel"));
    let (_, how) = running_seg(&[3, 4], 50).unwrap();
    assert_eq!(how.as_deref(), Some("/stop #3"), "compact, never gone");
    assert!(running_seg(&[], 120).is_none());
}

/// Names breathe like every other joiner on the footer.
#[test]
fn roster_names_are_joined_with_a_spaced_dot() {
    let agents = [agent("planner"), agent("coder"), agent("tester")];
    assert_eq!(
        roster_seg(&agents, 120).as_deref(),
        Some("planner \u{00b7} coder \u{00b7} tester")
    );
    assert_eq!(
        roster_seg(&agents, 30).as_deref(),
        Some("3 agents"),
        "narrow: the count"
    );
}
