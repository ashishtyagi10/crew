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
fn a_set_list_is_walked_in_place_of_the_ladder() {
    let m = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let set = m(&["claude-sonnet-4-5", "claude-haiku-4-5"]);
    assert_eq!(
        tail("claude-opus-5-5", &set),
        set,
        "never claude-sonnet-5-5"
    );
    let pricey = m(&["claude-opus-5-5", "claude-sonnet-5-5"]);
    assert!(
        tail("claude-haiku-5-5", &pricey).is_empty(),
        "a list does not lift a cheap ask"
    );
}

#[test]
fn with_no_list_only_the_ladders_own_ids_step() {
    assert!(
        tail("claude-opus-4-1", &[]).is_empty(),
        "no jump of generation"
    );
    assert!(tail("claude-sonnet-4-5-20250929", &[]).is_empty());
}

#[test]
fn the_swarm_reads_the_list_the_broker_does() {
    let p = || AnthropicProvider::new("k".into());
    assert_eq!(
        configured(p(), Some(" claude-sonnet-4-5 ,, claude-haiku-4-5".into())).chain,
        ["claude-sonnet-4-5", "claude-haiku-4-5"]
    );
    assert!(configured(p(), None).chain.is_empty());
}

#[test]
fn only_the_models_own_failures_walk() {
    let api = |b: &str| ProviderError::Api(b.into());
    assert!(walks_on(&api(OVERLOADED)));
    assert!(walks_on(&api(GONE)));
    let kind = |k: &str| api(&format!(r#"{{"type":"error","error":{{"type":"{k}"}}}}"#));
    assert!(walks_on(&kind("rate_limit_error")) && walks_on(&kind("api_error")));
    for code in [404, 429, 500, 529] {
        let s = format!("HTTP {code} from api.anthropic.com");
        assert!(walks_on(&ProviderError::Status(s)), "{code}");
    }
    assert!(!walks_on(&api(KEY)));
    assert!(!walks_on(&api(CREDIT)));
    let denied = r#"{"type":"error","error":{"type":"permission_error","message":"no"}}"#;
    assert!(!walks_on(&api(denied)));
    assert!(!walks_on(&ProviderError::Status(
        "HTTP 403 from api.anthropic.com: Forbidden".into()
    )));
    assert!(!walks_on(&ProviderError::Http("connection refused".into())));
}
