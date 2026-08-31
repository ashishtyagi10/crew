use super::*;

#[test]
fn keywords_map_to_their_strengths() {
    assert_eq!(strength_of("off"), Some(0));
    assert_eq!(strength_of("light"), Some(40));
    assert_eq!(strength_of("medium"), Some(70));
    assert_eq!(strength_of("heavy"), Some(120));
    assert_eq!(strength_of("glassy"), None);
}

#[test]
fn labels_name_the_ladder_and_number_the_rest() {
    assert_eq!(label_of(0), "off");
    assert_eq!(label_of(70), "medium");
    assert_eq!(
        label_of(crew_render::DEFAULT_SMOOTH),
        "off",
        "the default is a named step"
    );
    assert_eq!(label_of(42), "42");
}

#[test]
fn cycle_wraps_both_ways_and_adopts_custom_values() {
    assert_eq!(cycle(0, false), 40);
    assert_eq!(cycle(120, false), 0, "forward wraps heavy → off");
    assert_eq!(cycle(0, true), 120, "backward wraps off → heavy");
    assert_eq!(cycle(42, false), 40, "custom joins from the ladder top");
}
