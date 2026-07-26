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
    // `plugins::load` now honours `CREW_PROJECT_DIR` (`plugins::base_dir`,
    // mirroring `specialists`/`sessionlog`), so the manifest goes in our own
    // isolated dir rather than the crate's real `./.crew/agents` — writing
    // there (as this test used to, independent of `CREW_PROJECT_DIR`) landed
    // a real file in the working tree that a `cargo test` run would leave
    // behind on failure, and even on success left an empty `.crew/` (the
    // cleanup only removed `.crew/agents`, not its parent). `testenv::
    // no_provider`'s own dir is private to `mod.rs`, so this test points
    // `CREW_PROJECT_DIR` at a fresh dir of its own instead.
    let base =
        std::env::temp_dir().join(format!("crew-discover-plugin-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let agents_dir = base.join(".crew").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    std::env::set_var("CREW_PROJECT_DIR", &base);
    std::fs::write(
        agents_dir.join("regression-5c-probe.json"),
        r#"{"name":"regression-5c-probe","command":"sh","args":["-c","cat"],"role":"probe"}"#,
    )
    .unwrap();
    let agents = roster_with(&std::collections::HashMap::new());
    let names: Vec<String> = agents.iter().map(|a| a.name().to_string()).collect();
    assert!(
        names.contains(&"regression-5c-probe".to_string()),
        "plugin-only roster missing with no provider: {names:?}"
    );
}

/// The guard's promise ("even on a machine that exports a real key") has to
/// cover the credential store too: `forced_provider()` reads
/// `credentials::load()`, which resolves through `CREW_CREDENTIALS_PATH`.
/// Simulate a machine where the in-app key popup already saved a pin —
/// exactly what a real `~/.config/crew/credentials.json` would hold — by
/// pointing that override at a store we control (never the real path, via
/// `save_key_at`) BEFORE the guard exists, so `no_provider()` has to capture
/// and override it out from under us, same as the other four keys. If
/// `no_provider()` stopped setting `CREW_CREDENTIALS_PATH`, this stored pin
/// would leak straight through and `forced_provider()` would return
/// `Some("anthropic")` instead of `None`.
#[test]
fn no_provider_also_neutralises_a_stored_credential_pin() {
    // A fresh subdirectory, not a file directly under the shared system temp
    // dir: `save_key_at` chmods its parent 0o700, which the shared temp dir
    // itself will refuse.
    let fake_dir = std::env::temp_dir().join(format!(
        "crew-no-provider-cred-guard-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&fake_dir);
    let fake_store = fake_dir.join("credentials.json");
    crate::credentials::save_key_at(
        &fake_store,
        "ANTHROPIC_API_KEY",
        "sk-test-not-a-real-key",
        Some("anthropic"),
    )
    .unwrap();
    std::env::set_var("CREW_CREDENTIALS_PATH", &fake_store);

    let _env = testenv::no_provider();
    assert_eq!(
        forced_provider(),
        None,
        "no_provider() must neutralise a stored credential pin, not just the four env vars"
    );
    drop(_env);

    assert_eq!(
        std::env::var("CREW_CREDENTIALS_PATH").as_deref(),
        Ok(fake_store.to_str().unwrap()),
        "CREW_CREDENTIALS_PATH must be restored to its pre-guard value on drop, same as the others"
    );

    std::env::remove_var("CREW_CREDENTIALS_PATH");
    let _ = std::fs::remove_dir_all(&fake_dir);
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
    let default = default_openrouter_chain();
    let chain = parse_model_chain(None, default.clone());
    assert_eq!(chain.len(), default.len());
    assert_eq!(chain[0], default[0]);
}

#[test]
fn default_openrouter_chain_matches_the_catalogs_free_rows_in_order() {
    // Regression: the chain used to restate the catalog's four free ids
    // independently, so a rotation had to be fixed in two places. Now it's
    // derived, so this pins the actual shipped chain. A catalog edit that
    // reorders, drops, or mis-tags a free row must update this literal list
    // and re-verify against OpenRouter's live `/models` endpoint — the sole
    // guard against silent id retirement.
    let expected = [
        "nvidia/nemotron-3-ultra-550b-a55b:free",
        "openai/gpt-oss-20b:free",
        "google/gemma-4-31b-it:free",
        "cohere/north-mini-code:free",
    ];
    assert_eq!(
        default_openrouter_chain(),
        expected.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn model_chain_parses_comma_separated_override() {
    let chain = parse_model_chain(Some(" a:free , b:free ,, c ".into()), vec!["x".to_string()]);
    assert_eq!(chain, vec!["a:free", "b:free", "c"]); // trimmed, empties dropped
}

#[test]
fn model_chain_falls_back_to_default_when_blank() {
    assert_eq!(
        parse_model_chain(Some("  ,  ".into()), vec!["x".to_string(), "y".to_string()]),
        vec!["x", "y"]
    );
}

#[test]
fn the_env_pin_outranks_the_stored_pin() {
    assert_eq!(
        resolve_forced(
            Some("openrouter".to_string()),
            Some("anthropic".to_string())
        )
        .as_deref(),
        Some("openrouter"),
        "an explicit CREW_PROVIDER=… crew must never be overridden by a stored pin"
    );
}

#[test]
fn the_stored_pin_applies_when_the_env_is_unset_or_blank() {
    assert_eq!(
        resolve_forced(None, Some("anthropic".to_string())).as_deref(),
        Some("anthropic")
    );
    assert_eq!(
        resolve_forced(Some(String::new()), Some("anthropic".to_string())).as_deref(),
        Some("anthropic"),
        "a blank CREW_PROVIDER is not a pin"
    );
}

#[test]
fn no_pin_anywhere_leaves_auto_discovery_alone() {
    assert_eq!(resolve_forced(None, None), None);
    assert_eq!(resolve_forced(None, Some(String::new())), None);
}
