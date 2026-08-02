use super::*;

fn tmp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "crew-tokens-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    p.join("tokens.json")
}

fn set(access: &str) -> StoredToken {
    StoredToken {
        access: access.into(),
        refresh: Some("rt-x".into()),
        expires_at: 12345,
        resource: Some("https://portal/v1".into()),
    }
}

#[test]
fn store_load_round_trips_per_provider() {
    let p = tmp("roundtrip");
    store_at(&p, "dashscope", set("at-a")).unwrap();
    store_at(&p, "other", set("at-b")).unwrap();
    let a = load_at(&p, "dashscope").unwrap();
    assert_eq!(a.access, "at-a");
    assert_eq!(a.refresh.as_deref(), Some("rt-x"));
    assert_eq!(a.expires_at, 12345);
    assert_eq!(load_at(&p, "other").unwrap().access, "at-b");
    assert!(load_at(&p, "absent").is_none());
    clear_at(&p, "dashscope");
    assert!(load_at(&p, "dashscope").is_none(), "cleared grant is gone");
    assert!(
        load_at(&p, "other").is_some(),
        "clear touches one provider only"
    );
}

/// The file is owner-only from its first byte (via `credentials::write_atomic`).
#[cfg(unix)]
#[test]
fn the_token_file_is_created_0600() {
    use std::os::unix::fs::PermissionsExt;
    let p = tmp("mode");
    store_at(&p, "dashscope", set("at-secret")).unwrap();
    let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "token file mode must be 0600, got {mode:o}");
    let dir = std::fs::metadata(p.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(dir, 0o700, "token dir mode must be 0700, got {dir:o}");
}

#[test]
fn a_broken_file_reads_as_no_grant() {
    let p = tmp("broken");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "not json {{{").unwrap();
    assert!(load_at(&p, "dashscope").is_none());
    // …and storing over it recovers.
    store_at(&p, "dashscope", set("at-r")).unwrap();
    assert_eq!(load_at(&p, "dashscope").unwrap().access, "at-r");
}

#[test]
fn expiry_applies_the_skew_and_no_expiry_never_expires() {
    let granted = crew_hive::deviceflow::TokenSet {
        access_token: "at".into(),
        refresh_token: None,
        expires_in: Some(3600),
        resource_url: None,
    };
    let st = stored_from(&granted, 1_000);
    assert_eq!(st.expires_at, 1_000 + 3600 - EXPIRY_SKEW_SECS);
    assert!(is_fresh(&st, 1_000));
    assert!(!is_fresh(&st, st.expires_at), "expiry moment is stale");
    let forever = crew_hive::deviceflow::TokenSet {
        access_token: "at".into(),
        refresh_token: None,
        expires_in: None,
        resource_url: None,
    };
    assert_eq!(stored_from(&forever, 1_000).expires_at, u64::MAX);
}

#[test]
fn a_stored_token_debug_prints_no_secret() {
    let dbg = format!("{:?}", set("at-very-secret"));
    assert!(!dbg.contains("at-very-secret"), "{dbg}");
    assert!(!dbg.contains("rt-x"), "{dbg}");
    assert!(dbg.contains("<redacted>"), "{dbg}");
}
