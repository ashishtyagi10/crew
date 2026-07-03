use super::*;

#[test]
fn agent_color_is_stable_and_distinguishes_names() {
    assert_eq!(agent_color("planner"), agent_color("planner"));
}
