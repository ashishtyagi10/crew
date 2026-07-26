use super::*;

/// A unique scratch path per test — the store is a file, and the real config
/// directory must never be touched by a test run.
fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crew-cred-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("credentials.json")
}

#[test]
fn save_then_load_round_trips_the_key_and_pin() {
    let p = scratch("roundtrip");
    save_key_at(
        &p,
        "ANTHROPIC_API_KEY",
        "sk-test-not-a-real-key",
        Some("anthropic"),
    )
    .unwrap();
    let s = load_from(&p);
    assert_eq!(
        s.keys.get("ANTHROPIC_API_KEY").map(String::as_str),
        Some("sk-test-not-a-real-key")
    );
    assert_eq!(s.provider.as_deref(), Some("anthropic"));
}

#[test]
fn a_second_key_joins_the_first_and_moves_the_pin() {
    let p = scratch("second");
    save_key_at(&p, "ANTHROPIC_API_KEY", "sk-a", Some("anthropic")).unwrap();
    save_key_at(&p, "DASHSCOPE_API_KEY", "sk-d", Some("dashscope")).unwrap();
    let s = load_from(&p);
    assert_eq!(
        s.keys.len(),
        2,
        "the first key must survive the second save"
    );
    assert_eq!(s.provider.as_deref(), Some("dashscope"));
}

#[test]
fn an_empty_value_removes_rather_than_stores_a_blank() {
    // A blank ANTHROPIC_API_KEY is exactly the trap that outranks a valid
    // OAuth profile — the store must never hold one.
    let p = scratch("empty");
    save_key_at(&p, "ANTHROPIC_API_KEY", "sk-a", None).unwrap();
    save_key_at(&p, "ANTHROPIC_API_KEY", "", None).unwrap();
    assert!(!load_from(&p).keys.contains_key("ANTHROPIC_API_KEY"));
}

#[test]
fn an_unknown_variable_is_refused() {
    let p = scratch("unknown");
    let err = save_key_at(&p, "AWS_SECRET_ACCESS_KEY", "x", None).unwrap_err();
    assert!(err.to_string().contains("AWS_SECRET_ACCESS_KEY"));
    assert!(!p.exists(), "a refused write must not create the file");
}

#[test]
fn a_missing_or_malformed_file_loads_as_default() {
    let p = scratch("malformed");
    assert_eq!(load_from(&p), Store::default(), "missing file");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "{ this is not json").unwrap();
    assert_eq!(
        load_from(&p),
        Store::default(),
        "malformed file must not break startup"
    );
}

#[test]
fn the_write_is_atomic_and_leaves_no_temp_file() {
    let p = scratch("atomic");
    save_key_at(&p, "OPENROUTER_API_KEY", "sk-o", None).unwrap();
    let leftovers: Vec<_> = std::fs::read_dir(p.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "temp file left behind: {leftovers:?}");
}

#[cfg(unix)]
#[test]
fn the_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let p = scratch("perms");
    save_key_at(&p, "OPENROUTER_API_KEY", "sk-o", None).unwrap();
    let file = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
    let dir = std::fs::metadata(p.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(file, 0o600, "credentials file must be owner-only");
    assert_eq!(dir, 0o700, "credentials dir must be owner-only");
}

#[test]
fn provider_for_maps_every_var_and_nothing_else() {
    assert_eq!(provider_for("DASHSCOPE_API_KEY"), Some("dashscope"));
    assert_eq!(provider_for("OPENROUTER_API_KEY"), Some("openrouter"));
    assert_eq!(provider_for("ANTHROPIC_API_KEY"), Some("anthropic"));
    assert_eq!(provider_for("AWS_SECRET_ACCESS_KEY"), None);
    for v in VARS {
        assert!(provider_for(v).is_some(), "{v} has no provider mapping");
    }
}
