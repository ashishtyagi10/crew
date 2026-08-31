use super::*;

#[test]
fn model_id_cheap() {
    assert_eq!(ModelTier::Cheap.model_id(), "claude-haiku-4-5");
}

#[test]
fn model_id_standard() {
    assert_eq!(ModelTier::Standard.model_id(), "claude-sonnet-4-6");
}

#[test]
fn model_id_capable() {
    assert_eq!(ModelTier::Capable.model_id(), "claude-opus-4-8");
}
