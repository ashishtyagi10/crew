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

/// A store on disk, seeded through `save_key_at` (never the real
/// `credentials::path()`), in a fresh subdirectory of its own: `save_key_at`
/// chmods its parent 0o700, which the shared system temp dir itself refuses.
/// Returns `(dir, store_path)`; the caller removes `dir` when done.
fn seeded_store(
    tag: &str,
    var: &str,
    value: &str,
    pin: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("crew-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = dir.join("credentials.json");
    crate::credentials::save_key_at(&store, var, value, Some(pin)).unwrap();
    (dir, store)
}

/// The guard's promise ("even on a machine that exports a real key") has to
/// cover the credential store too: `forced_provider()` reads
/// `credentials::load()`, which resolves through `CREW_CREDENTIALS_PATH`.
/// Simulate a machine where the in-app key popup already saved a pin —
/// exactly what a real `~/.config/crew/credentials.json` would hold — with
/// `no_provider_masking`, which installs that path and then has to override it
/// out from under us, same as the other four keys. If `no_provider()` stopped
/// neutralising `CREW_CREDENTIALS_PATH`, this stored pin would leak straight
/// through and `forced_provider()` would return `Some("anthropic")` instead of
/// `None`.
///
/// Every write to the process environment here happens inside the guard's own
/// lock (that is what `no_provider_masking` is for): this test used to
/// `set_var`/`remove_var` around the guard, racing the hundreds of other tests
/// in this binary.
#[test]
fn no_provider_also_neutralises_a_stored_credential_pin() {
    let (fake_dir, fake_store) = seeded_store(
        "no-provider-cred-guard",
        "ANTHROPIC_API_KEY",
        "sk-test-not-a-real-key",
        "anthropic",
    );

    let env = testenv::no_provider_masking(&fake_store);
    assert_eq!(
        forced_provider(&crate::credentials::load()),
        None,
        "no_provider() must neutralise a stored credential pin, not just the four env vars"
    );
    assert!(
        env.restores("CREW_CREDENTIALS_PATH"),
        "CREW_CREDENTIALS_PATH must be captured for restore on drop, same as the others"
    );
    drop(env);

    let _ = std::fs::remove_dir_all(&fake_dir);
}

/// C1: a key typed into the in-app popup lands in the credential store, never
/// in this process's environment — and the broker child was already running
/// when it was saved, so `shellenv::hydrate()`'s one-shot import never saw it.
/// The provider/key resolution therefore has to read the store per request,
/// exactly as the pin already is.
///
/// Before the fix this returned `None`: `picked` resolved DashScope from the
/// stored pin, then the key lookup read only `std::env::var`, found nothing,
/// and `roster_with` collapsed to plugins only — the user's specialists
/// vanishing the moment they saved a key.
#[test]
fn a_stored_key_resolves_a_provider_with_nothing_in_the_environment() {
    let (dir, store) = seeded_store(
        "stored-key-resolves",
        "DASHSCOPE_API_KEY",
        "sk-fake-not-a-real-key",
        "dashscope",
    );
    let env = testenv::no_provider_with_store(&store);
    assert_eq!(
        picked(&crate::credentials::load()),
        Some(ProviderKind::DashScope),
        "the stored pin picks the provider"
    );
    let resolved = provider_and_model_for(crew_hive::ModelTier::Standard);
    assert!(
        resolved.is_some(),
        "a stored key must resolve a provider with an empty environment — \
         without it roster_with returns plugins only"
    );
    drop(env);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_exported_key_still_outranks_the_stored_one() {
    // The precedence `shellenv::credential_imports` already applies, restated
    // on the read side: an exported variable is the most deliberate signal a
    // user can send and must never be shadowed by something crew stored.
    assert_eq!(
        resolve_key_with(
            Some("sk-from-the-environment".to_string()),
            Some("sk-from-the-store".to_string()),
            false
        )
        .as_deref(),
        Some("sk-from-the-environment")
    );
    assert_eq!(
        resolve_key_with(None, Some("sk-from-the-store".to_string()), false).as_deref(),
        Some("sk-from-the-store")
    );
    assert_eq!(
        resolve_key_with(
            Some(String::new()),
            Some("sk-from-the-store".to_string()),
            false
        )
        .as_deref(),
        Some("sk-from-the-store"),
        "an empty export is not a key"
    );
    assert_eq!(resolve_key_with(None, None, false), None);
    assert_eq!(resolve_key_with(None, Some(String::new()), false), None);
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

#[test]
fn a_rotated_key_beats_crews_own_startup_injection() {
    // shellenv::hydrate copies the store into the env once per broker process,
    // but the store is re-read every request. Without this, pasting a
    // replacement key in a later session would update the store and change
    // nothing — crew's own stale injection would win every request.
    assert_eq!(
        resolve_key_with(Some("sk-old".into()), Some("sk-new".into()), true).as_deref(),
        Some("sk-new")
    );
}

#[test]
fn a_user_exported_key_still_beats_the_store() {
    // The precedence that must NOT change: an explicit export is the most
    // deliberate signal a user can send.
    assert_eq!(
        resolve_key_with(Some("sk-env".into()), Some("sk-stored".into()), false).as_deref(),
        Some("sk-env")
    );
}

#[test]
fn a_crew_injected_key_falls_back_to_the_env_when_the_store_is_emptied() {
    // Degrade to the old value rather than to no provider at all.
    assert_eq!(
        resolve_key_with(Some("sk-old".into()), None, true).as_deref(),
        Some("sk-old")
    );
    assert_eq!(
        resolve_key_with(Some("sk-old".into()), Some(String::new()), true).as_deref(),
        Some("sk-old")
    );
}

#[test]
fn nothing_anywhere_resolves_to_nothing() {
    assert_eq!(resolve_key_with(None, None, true), None);
    assert_eq!(resolve_key_with(Some(String::new()), None, false), None);
}

/// A table provider is discovered from its own key — but LAST, so adding a
/// row can never change which provider an existing install resolves to.
#[test]
fn a_direct_provider_is_found_by_its_key_but_never_outranks_the_originals() {
    let openai = super::direct_by_name("openai").expect("openai row");
    let only_openai = |k: &str| k == "OPENAI_API_KEY";
    assert_eq!(
        pick_provider(None, only_openai),
        Some(ProviderKind::Direct(openai))
    );
    // With an older key alongside it, the older key still wins.
    let both = |k: &str| k == "OPENAI_API_KEY" || k == "DASHSCOPE_API_KEY";
    assert_eq!(pick_provider(None, both), Some(ProviderKind::DashScope));
}

/// `CREW_PROVIDER=openai` pins it, exactly as the three original names do.
#[test]
fn a_direct_provider_can_be_forced_by_name() {
    let openai = super::direct_by_name("openai").expect("openai row");
    assert_eq!(
        pick_provider(Some("openai"), |_| false),
        Some(ProviderKind::Direct(openai))
    );
    assert_eq!(pick_provider(Some("nope"), |_| false), None);
}

/// Every table row must be storable, or its key popup could never save one.
#[test]
fn every_table_row_is_a_known_credential_variable() {
    for d in super::DIRECT {
        assert!(
            crate::credentials::VARS.contains(&d.var),
            "{} is not in credentials::VARS",
            d.var
        );
        assert_eq!(
            crate::credentials::provider_for(d.var),
            Some(d.name),
            "storing {} must pin {}",
            d.var,
            d.name
        );
    }
}

/// The user-facing name. `/doctor` printed `direct(direct(openai))` when this
/// came from the `Debug` spelling — a nested variant is a fine debug string
/// and a terrible report line.
#[test]
fn provider_names_read_as_names() {
    assert_eq!(ProviderKind::OpenRouter.name(), "openrouter");
    assert_eq!(ProviderKind::Anthropic.name(), "anthropic");
    let openai = super::direct_by_name("openai").expect("openai row");
    assert_eq!(ProviderKind::Direct(openai).name(), "openai");
    // The name must round-trip through `CREW_PROVIDER`, or a report would
    // print something the user cannot type back.
    for d in super::DIRECT {
        assert_eq!(
            pick_provider(Some(d.name), |_| false),
            Some(ProviderKind::Direct(d))
        );
    }
}

/// No invented model ids. Every slug a provider row would send must already
/// exist in the catalog as a NATIVE slug for that vendor — free rows are
/// excluded because their ids are OpenRouter-shaped (`openai/gpt-oss-20b:free`)
/// and api.openai.com has never heard of them.
#[test]
fn chains_are_native_catalog_slugs() {
    for d in super::DIRECT {
        assert!(!d.chain.is_empty(), "{} has no default model", d.name);
        for slug in d.chain {
            let known = crew_hive::catalog::catalog().iter().any(|m| {
                m.vendor == d.vendor && m.slug == *slug && !m.free && !m.slug.contains('/')
            });
            assert!(
                known,
                "{}: {slug} is not a native catalog slug for {:?} — a default \
                 model that 404s on first use is worse than no provider",
                d.name, d.vendor
            );
        }
    }
}

/// Each row must be reachable and distinct: a duplicated name or variable
/// would make one of them permanently unselectable.
#[test]
fn provider_rows_are_distinct_and_addressable() {
    let mut names: Vec<&str> = Vec::new();
    let mut vars: Vec<&str> = Vec::new();
    for d in super::DIRECT {
        assert!(!names.contains(&d.name), "duplicate provider {}", d.name);
        assert!(!vars.contains(&d.var), "duplicate variable {}", d.var);
        assert!(
            d.endpoint.starts_with("https://"),
            "{} must be https",
            d.name
        );
        names.push(d.name);
        vars.push(d.var);
        assert_eq!(super::direct_by_name(d.name), Some(d));
    }
}

/// Choosing a model is choosing a provider. With an Anthropic key held but
/// another provider active, picking a Claude model moves the pin — otherwise
/// the fixed discovery order would keep answering with the wrong vendor and
/// the user would have to know `CREW_PROVIDER` exists.
#[test]
fn picking_a_model_moves_the_pin_to_whoever_can_serve_it() {
    let _env = crate::broker::testenv::no_provider();
    // The guard points CREW_CREDENTIALS_PATH at a per-test store; write
    // through the same path the code under test will read.
    let store = crate::credentials::path().expect("guarded store path");
    crate::credentials::save_key_at(&store, "ANTHROPIC_API_KEY", "sk-a", Some("anthropic"))
        .unwrap();
    crate::credentials::save_pin_at(&store, "openrouter").unwrap();
    // Anthropic model, anthropic key held, openrouter pinned → switch.
    assert_eq!(
        super::pin_provider_for_model("claude-sonnet-5"),
        Some("anthropic")
    );
    assert_eq!(
        crate::credentials::load_from(&store).provider.as_deref(),
        Some("anthropic")
    );
    // Already active → nothing to move, and no needless write.
    assert_eq!(super::pin_provider_for_model("claude-sonnet-5"), None);
}

/// An OpenRouter-shaped id is a routing instruction in itself and must never
/// drag the pin somewhere else; an unknown slug is nobody's business.
#[test]
fn slugs_that_name_no_direct_vendor_leave_the_pin_alone() {
    let _env = crate::broker::testenv::no_provider();
    assert_eq!(
        super::pin_provider_for_model("anthropic/claude-sonnet-5"),
        None
    );
    assert_eq!(super::pin_provider_for_model("not-a-real-model"), None);
}

/// Every `DIRECT` row is a provider a user can actually reach — a key var, an
/// endpoint and a model chain — and a provider nobody is told about is a
/// provider nobody uses. `openai`, `gemini` and `deepseek` each shipped fully
/// wired and entirely undocumented, so the table now has to answer to the
/// manual: add a row without a paragraph and this fails, which is the only
/// mechanism that has ever kept two lists together in this codebase (see
/// `broker_constructs`).
#[test]
fn every_direct_provider_is_documented() {
    const MANUAL: &str = include_str!("../../../../docs/CREW.md");
    for d in DIRECT {
        for needle in [d.name, d.var, d.chain_env, d.base_url_env] {
            assert!(
                MANUAL.contains(needle),
                "docs/CREW.md never mentions `{needle}` \u{2014} provider `{}` is \
                 wired but undocumented",
                d.name
            );
        }
    }
}

/// `CREW_PROVIDER` has accepted a `DIRECT` name since the table landed, but
/// the docs (and `pick_provider`'s own comment) listed only the original
/// three, so the pin looked impossible for exactly the providers that most
/// needed pinning — they probe last by design.
#[test]
fn crew_provider_pins_a_direct_row_by_name() {
    for d in DIRECT {
        assert_eq!(
            pick_provider(Some(d.name), keys(&[])),
            Some(ProviderKind::Direct(d)),
            "CREW_PROVIDER={} should pin the direct row even with no key set",
            d.name
        );
    }
}
