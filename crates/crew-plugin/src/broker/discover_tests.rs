use super::*;
use crate::broker::testenv;

fn keys(set: &'static [&'static str]) -> impl Fn(&str) -> bool {
    move |k| set.contains(&k)
}

/// A manifest plugin agent needs no API key (it shells out to an installed
/// CLI), so a project with zero provider keys but an installed plugin must
/// still get a working, plugin-only roster — not an empty one. Regression for
/// the bug where `roster_with` early-returned `Vec::new()` before ever
/// reaching `plugins::append` when no provider resolved.
#[test]
fn roster_with_falls_back_to_plugins_when_no_provider_resolves() {
    let _env = testenv::no_provider();
    // `plugins::load` reads `./.crew/agents` relative to the process cwd
    // (cargo's test cwd is the crate root), independent of `CREW_PROJECT_DIR`.
    let agents_dir = std::path::Path::new(".crew/agents");
    std::fs::create_dir_all(agents_dir).unwrap();
    let manifest_path = agents_dir.join("regression-5c-probe.json");
    std::fs::write(
        &manifest_path,
        r#"{"name":"regression-5c-probe","command":"sh","args":["-c","cat"],"role":"probe"}"#,
    )
    .unwrap();
    let cleanup = || {
        let _ = std::fs::remove_file(&manifest_path);
        let _ = std::fs::remove_dir(agents_dir); // best-effort; only if now empty
    };
    let agents = roster_with(&std::collections::HashMap::new());
    cleanup();
    let names: Vec<String> = agents.iter().map(|a| a.name().to_string()).collect();
    assert!(
        names.contains(&"regression-5c-probe".to_string()),
        "plugin-only roster missing with no provider: {names:?}"
    );
}

#[test]
fn pick_prefers_dashscope_over_openrouter() {
    let has = keys(&[
        "DASHSCOPE_API_KEY",
        "OPENROUTER_API_KEY",
        "ANTHROPIC_API_KEY",
    ]);
    assert_eq!(pick_provider(None, has), Some(ProviderKind::DashScope));
}

#[test]
fn pick_auto_order_openrouter_then_anthropic() {
    let has = keys(&["OPENROUTER_API_KEY", "ANTHROPIC_API_KEY"]);
    assert_eq!(pick_provider(None, has), Some(ProviderKind::OpenRouter));
    let has = keys(&["ANTHROPIC_API_KEY"]);
    assert_eq!(pick_provider(None, has), Some(ProviderKind::Anthropic));
    assert_eq!(pick_provider(None, keys(&[])), None);
}

#[test]
fn pick_forced_provider_beats_auto_order() {
    let has = keys(&["DASHSCOPE_API_KEY", "OPENROUTER_API_KEY"]);
    assert_eq!(
        pick_provider(Some("openrouter"), has),
        Some(ProviderKind::OpenRouter)
    );
    // Case-insensitive; unknown values fall back to auto.
    let has = keys(&["DASHSCOPE_API_KEY", "OPENROUTER_API_KEY"]);
    assert_eq!(
        pick_provider(Some("Anthropic"), has),
        Some(ProviderKind::Anthropic)
    );
    let has = keys(&["DASHSCOPE_API_KEY"]);
    assert_eq!(
        pick_provider(Some("bogus"), has),
        Some(ProviderKind::DashScope)
    );
}

#[test]
fn pick_mock_beats_everything() {
    let has = keys(&["CREW_BROKER_MOCK_REPLY", "DASHSCOPE_API_KEY"]);
    assert_eq!(
        pick_provider(Some("dashscope"), has),
        Some(ProviderKind::Mock)
    );
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
