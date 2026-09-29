use super::*;

#[test]
fn model_id_cheap() {
    assert_eq!(ModelTier::Cheap.model_id(), "claude-haiku-4-5");
}

#[test]
fn model_id_standard() {
    assert_eq!(ModelTier::Standard.model_id(), "claude-sonnet-5-5");
}

#[test]
fn model_id_capable() {
    assert_eq!(ModelTier::Capable.model_id(), "claude-opus-5-5");
}

/// Every tier's id is one the curated catalog lists and the pricing table
/// prices: a tier naming a model neither knows would bill its calls at $0.
#[test]
fn every_tier_names_a_catalogued_model() {
    for tier in [ModelTier::Cheap, ModelTier::Standard, ModelTier::Capable] {
        let id = tier.model_id();
        assert!(
            crate::catalog::catalog().iter().any(|m| m.slug == id),
            "{tier:?} sends {id}, which the catalog does not list"
        );
        assert!(crate::pricing::rate(id).is_some(), "{id} is unpriced");
    }
}
