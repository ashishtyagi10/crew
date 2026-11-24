//! Which failures walk the Anthropic chain, and which way the ladder runs.
use super::tests::{CREDIT, GONE, KEY, OVERLOADED};
use super::*;

#[test]
fn the_ladder_only_steps_down() {
    let m = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        tail("claude-opus-5-5", &[]),
        m(&["claude-sonnet-5-5", "claude-haiku-5-5"])
    );
    assert_eq!(tail("claude-sonnet-5-5", &[]), m(&["claude-haiku-5-5"]));
    assert!(
        tail("claude-haiku-5-5", &[]).is_empty(),
        "a cheap ask never falls UP"
    );
    let set = m(&["claude-opus-5-5", "claude-sonnet-4-5"]);
    assert_eq!(tail("claude-opus-5-5", &set), m(&["claude-sonnet-4-5"]));
    assert!(tail("claude-sonnet-4-5", &set).is_empty());
}

#[test]
fn only_the_models_own_failures_walk() {
    let api = |b: &str| ProviderError::Api(b.into());
    assert!(walks_on(&api(OVERLOADED)));
    assert!(walks_on(&api(GONE)));
    assert!(walks_on(&ProviderError::Status(
        "HTTP 529 from api.anthropic.com".into()
    )));
    assert!(!walks_on(&api(KEY)));
    assert!(!walks_on(&api(CREDIT)));
    let denied = r#"{"type":"error","error":{"type":"permission_error","message":"no"}}"#;
    assert!(!walks_on(&api(denied)));
    assert!(!walks_on(&ProviderError::Status(
        "HTTP 403 from api.anthropic.com: Forbidden".into()
    )));
    assert!(!walks_on(&ProviderError::Http("connection refused".into())));
}
