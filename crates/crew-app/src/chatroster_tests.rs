use super::*;

#[test]
fn agent_color_is_stable_and_distinguishes_names() {
    let _g = crate::app::theme_test_guard();
    assert_eq!(agent_color("planner"), agent_color("planner"));
}
