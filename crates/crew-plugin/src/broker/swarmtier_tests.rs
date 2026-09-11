use super::parse;
use crew_hive::ModelTier;

#[test]
fn the_swarm_serves_at_standard_unless_told_otherwise() {
    assert_eq!(parse(None), ModelTier::Standard);
    assert_eq!(parse(Some("")), ModelTier::Standard);
    assert_eq!(parse(Some("standard")), ModelTier::Standard);
    assert_eq!(parse(Some("nonsense")), ModelTier::Standard);
}

#[test]
fn cheap_flips_the_swarm_to_the_cheap_tier_whatever_its_case() {
    assert_eq!(parse(Some("cheap")), ModelTier::Cheap);
    assert_eq!(parse(Some(" CHEAP ")), ModelTier::Cheap);
}

#[test]
fn the_knob_can_only_make_crew_cheaper_never_dearer() {
    // `capable` is not offered: a swarm that runs Opus on every worker is a
    // bill nobody set out to pay by exporting one variable.
    assert_eq!(parse(Some("capable")), ModelTier::Standard);
}
