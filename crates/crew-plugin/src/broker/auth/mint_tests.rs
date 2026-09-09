use super::*;

#[test]
fn a_token_is_one_bare_word_from_a_successful_run() {
    assert_eq!(
        parse_token(true, "sk-ant-oat01-abc.DEF_9\n"),
        Some("sk-ant-oat01-abc.DEF_9".into())
    );
    assert_eq!(
        parse_token(false, "sk-ant-oat01-abc\n"),
        None,
        "a failed run"
    );
    assert_eq!(parse_token(true, ""), None);
    assert_eq!(parse_token(true, "   \n"), None);
    assert_eq!(
        parse_token(true, "not logged in (profile \"default\")"),
        None,
        "prose is not a token"
    );
    assert_eq!(
        parse_token(true, r#"{"access_token":"x"}"#),
        None,
        "the JSON form of print-credentials is not a token"
    );
    assert_eq!(parse_token(true, "tok-1\ntok-2\n"), None, "two lines");
}

/// One mint serves every request for the TTL — `key_for` runs per model
/// call and must not spawn a process each time — and a run that fails or
/// prints no token caches nothing, so the next call tries again.
#[test]
fn a_minted_token_is_cached_for_the_ttl_and_failures_are_not() {
    let mut cache = Cache::new();
    let calls = std::cell::Cell::new(0);
    let run = |bin: &str, args: &[&str]| {
        assert_eq!((bin, args), ("ant", ANT.mint));
        calls.set(calls.get() + 1);
        Some((true, format!("tok-{}\n", calls.get())))
    };
    let mut at = |t: u64| fresh_with(&mut cache, "anthropic", &ANT, t, &run);
    assert_eq!(at(1_000).as_deref(), Some("tok-1"));
    assert_eq!(at(1_000 + MINT_TTL_SECS - 1).as_deref(), Some("tok-1"));
    assert_eq!(calls.get(), 1, "the second call served from the cache");
    assert_eq!(at(1_000 + MINT_TTL_SECS).as_deref(), Some("tok-2"));
    assert_eq!(
        at(999).as_deref(),
        Some("tok-2"),
        "a clock that went backwards still serves"
    );

    let mut empty = Cache::new();
    let fail = |_: &str, _: &[&str]| Some((false, "not logged in".to_string()));
    assert_eq!(fresh_with(&mut empty, "anthropic", &ANT, 5, &fail), None);
    assert!(empty.is_empty(), "a failure caches nothing");
    let spawn_fail = |_: &str, _: &[&str]| None;
    assert_eq!(
        fresh_with(&mut empty, "anthropic", &ANT, 5, &spawn_fail),
        None
    );
    assert!(empty.is_empty());
}

/// The spec IS the documented CLI surface (ant 1.31.0, 2026-09-09): the
/// status command that never fails, its signed-in marker, the flag that
/// prints a bare token (the flagless form prints JSON), and the sign-out.
#[test]
fn the_ant_spec_is_the_documented_cli_surface() {
    assert_eq!(ANT.cli.bin, "ant");
    assert_eq!(ANT.cli.status, ["auth", "status"]);
    assert_eq!(ANT.cli.login, "ant auth login");
    assert_eq!(ANT.cli.signed_in_marker, Some("user_oauth"));
    assert_eq!(ANT.mint, ["auth", "print-credentials", "--access-token"]);
    assert_eq!(ANT.logout, "ant auth logout");
    assert!(ANT.install.contains("anthropics/tap/ant"));
}

/// The stand-in answers only for a var whose provider mints; a plain key
/// var never spawns anything.
#[test]
fn the_stand_in_ignores_unminted_vars() {
    assert_eq!(key_stand_in("OPENROUTER_API_KEY"), None);
    assert_eq!(key_stand_in("NOT_A_KEY"), None);
}
