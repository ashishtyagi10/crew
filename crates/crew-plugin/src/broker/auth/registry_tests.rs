use super::*;

#[test]
fn the_seed_entries_declare_the_designed_modes() {
    let e = |n: &str| by_name(n).unwrap_or_else(|| panic!("{n} missing"));
    assert!(e("claude-code").delegated() && !e("claude-code").keyed());
    assert!(e("codex").delegated() && !e("codex").keyed());
    // Qwen/DashScope: key or its permitted third-party device flow.
    assert!(e("dashscope").keyed());
    assert!(e("dashscope").modes.contains(&AuthMode::OauthDevice));
    assert!(e("dashscope").device.is_some());
    assert!(e("openrouter").keyed() && !e("openrouter").delegated());
    assert!(e("mock").modes == [AuthMode::Mock]);
}

/// The keyed order IS the historic `pick_provider` order — dashscope,
/// openrouter, anthropic, then every DIRECT row in table order. The
/// resolution rung iterates this list, so this order is load-bearing.
#[test]
fn keyed_entries_keep_the_historic_discovery_order() {
    let names: Vec<&str> = keyed().iter().map(|e| e.name).collect();
    let mut want = vec!["dashscope", "openrouter", "anthropic"];
    want.extend(crate::broker::discover::DIRECT.iter().map(|d| d.name));
    assert_eq!(names, want);
}

/// Every keyed entry's variable is one the credential store will hold —
/// otherwise a key typed into the UI could name a provider the store
/// refuses to save (or vice versa: the drift this test exists to catch).
#[test]
fn every_keyed_var_is_storable() {
    for e in keyed() {
        let var = e.key_var.expect("keyed entries carry a var");
        assert!(
            crate::credentials::VARS.contains(&var),
            "{var} missing from credentials::VARS"
        );
    }
}

#[test]
fn by_name_matches_registry_name_or_cli_binary_case_insensitively() {
    assert_eq!(by_name("Claude-Code").unwrap().name, "claude-code");
    assert_eq!(by_name("claude").unwrap().name, "claude-code");
    assert_eq!(by_name("CODEX").unwrap().name, "codex");
    assert_eq!(by_name("openai").unwrap().name, "openai");
    assert!(by_name("nope").is_none());
}

/// Delegated entries carry everything the two consent-based flows need: a
/// status introspection argv and the exact login command to hand the user.
#[test]
fn delegated_entries_carry_probe_and_login_commands() {
    let d = delegated();
    assert_eq!(
        d.iter().map(|e| e.cli.unwrap().bin).collect::<Vec<_>>(),
        ["claude", "codex"]
    );
    for e in &d {
        let cli = e.cli.unwrap();
        assert!(!cli.status.is_empty(), "{} has no status probe", e.name);
        assert!(cli.login.starts_with(cli.bin), "{} login command", e.name);
    }
}

/// Device endpoints exist exactly where `OauthDevice` is declared — an
/// endpoint on a provider whose terms forbid the flow would be rung zero.
#[test]
fn device_endpoints_pair_exactly_with_the_oauth_device_mode() {
    for e in entries() {
        assert_eq!(
            e.device.is_some(),
            e.modes.contains(&AuthMode::OauthDevice),
            "{}: device endpoints and OauthDevice mode must pair",
            e.name
        );
    }
    let d = by_name("dashscope").unwrap().device.unwrap();
    assert!(d.device_url.starts_with("https://"), "{}", d.device_url);
    assert!(d.token_url.starts_with("https://"), "{}", d.token_url);
    assert!(!d.client_id.is_empty() && !d.scope.is_empty());
}

#[test]
fn name_for_var_answers_from_the_table() {
    assert_eq!(name_for_var("DASHSCOPE_API_KEY"), Some("dashscope"));
    assert_eq!(name_for_var("OPENAI_API_KEY"), Some("openai"));
    assert_eq!(name_for_var("NOT_A_KEY"), None);
}
