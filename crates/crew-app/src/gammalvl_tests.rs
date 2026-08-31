use super::*;

#[test]
fn keywords_map_to_their_amounts() {
    assert_eq!(amount_of("off"), Some(0));
    assert_eq!(amount_of("light"), Some(65));
    assert_eq!(amount_of("medium"), Some(130));
    assert_eq!(amount_of("full"), Some(crew_render::DEFAULT_TEXT_GAMMA));
    assert_eq!(amount_of("heavy"), None);
}

#[test]
fn labels_name_the_ladder_and_number_the_rest() {
    assert_eq!(label_of(0), "off");
    assert_eq!(label_of(130), "medium");
    assert_eq!(
        label_of(crew_render::DEFAULT_TEXT_GAMMA),
        "full",
        "the default is a named step"
    );
    assert_eq!(label_of(42), "42");
}

#[test]
fn cycle_wraps_both_ways_and_adopts_custom_values() {
    assert_eq!(cycle(0, false), 65);
    assert_eq!(cycle(255, false), 0, "forward wraps full → off");
    assert_eq!(cycle(0, true), 255, "backward wraps off → full");
    assert_eq!(cycle(42, false), 65, "custom joins from the ladder top");
}
