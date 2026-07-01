use super::*;
use std::time::Duration;

struct Stub(&'static str);
impl Adapter for Stub {
    fn name(&self) -> &str {
        self.0
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _body: &str, _t: Duration) -> Result<String, String> {
        Ok(String::new())
    }
}

fn reg() -> Registry {
    Registry::new(vec![Box::new(Stub("claude")), Box::new(Stub("codex"))])
}

#[test]
fn get_is_case_insensitive() {
    assert!(reg().get("Claude").is_some());
    assert!(reg().get("nope").is_none());
}

#[test]
fn peers_excludes_self() {
    assert_eq!(reg().peers_of("claude"), vec!["codex".to_string()]);
}

#[test]
fn roster_excluding_adds_role_hints() {
    let roster = reg().roster_excluding("claude");
    assert_eq!(roster.len(), 1);
    assert!(roster[0].starts_with("codex ("), "{}", roster[0]);
}

#[test]
fn names_and_len() {
    let r = reg();
    assert_eq!(r.len(), 2);
    assert_eq!(r.names(), vec!["claude", "codex"]);
    assert!(!r.is_empty());
}

#[test]
fn infos_carry_name_role_and_model() {
    let infos = reg().infos();
    assert_eq!(infos.len(), 2);
    assert_eq!(infos[0].name, "claude");
    assert!(!infos[0].role.is_empty(), "known agents get a role hint");
    assert_eq!(infos[0].model, ""); // Stub uses the default (unknown) model
}

#[test]
fn model_chain_defaults_when_unset() {
    let chain = parse_model_chain(None, DEFAULT_OPENROUTER_CHAIN);
    assert_eq!(chain.len(), DEFAULT_OPENROUTER_CHAIN.len());
    assert_eq!(chain[0], DEFAULT_OPENROUTER_CHAIN[0]);
}

#[test]
fn model_chain_parses_comma_separated_override() {
    let chain = parse_model_chain(Some(" a:free , b:free ,, c ".into()), &["x"]);
    assert_eq!(chain, vec!["a:free", "b:free", "c"]); // trimmed, empties dropped
}

#[test]
fn model_chain_falls_back_to_default_when_blank() {
    assert_eq!(
        parse_model_chain(Some("  ,  ".into()), &["x", "y"]),
        vec!["x", "y"]
    );
}
