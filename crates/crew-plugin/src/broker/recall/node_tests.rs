use super::*;

#[test]
fn every_kind_and_relation_survives_a_round_trip_through_its_tag() {
    for k in [Kind::Topic, Kind::File, Kind::Turn] {
        assert_eq!(Kind::parse(k.tag()), Some(k), "{k} lost its tag");
    }
    for r in [Rel::Mentions, Rel::Then, Rel::With] {
        assert_eq!(Rel::parse(r.tag()), Some(r), "{} lost its tag", r.tag());
    }
}

#[test]
fn an_unknown_tag_is_none_rather_than_a_default_kind() {
    // A file written by a newer crew must drop the records it cannot read,
    // not silently file them under Topic.
    assert_eq!(Kind::parse("decision"), None);
    assert_eq!(Rel::parse("caused"), None);
}
