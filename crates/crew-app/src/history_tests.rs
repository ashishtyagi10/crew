use super::*;

#[test]
fn round_trips_and_filters_blanks() {
    let h = vec!["ls".to_string(), "cargo test".to_string()];
    assert_eq!(deserialize(&serialize(&h)), h);
    assert_eq!(
        deserialize("a\n\n b \n"),
        vec!["a".to_string(), " b ".to_string()]
    );
}

/// The history file can capture anything a user typed into the input bar,
/// including a secret typed into it by mistake. `std::fs::write` created
/// it 0644 — world-readable — with no mode ever set.
#[cfg(unix)]
#[test]
fn saved_history_is_owner_only_and_still_round_trips() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("history");
    save_to(&p, &["ls".to_string(), "cargo test".to_string()]);
    let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "history must not be readable by anyone else");
    assert_eq!(
        deserialize(&std::fs::read_to_string(&p).unwrap()),
        vec!["ls".to_string(), "cargo test".to_string()]
    );
    // Atomic write: no temp file left behind.
    assert!(!p.with_extension("tmp").exists());
}

/// Re-saving over an existing world-readable history (one written by an
/// older build) must leave it owner-only, not inherit the old mode.
#[cfg(unix)]
#[test]
fn saving_over_a_world_readable_history_tightens_it() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("history");
    std::fs::write(&p, "old\n").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
    save_to(&p, &["new".to_string()]);
    let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
}

#[test]
fn serialize_caps_to_max() {
    let h: Vec<String> = (0..MAX + 50).map(|i| i.to_string()).collect();
    let out = deserialize(&serialize(&h));
    assert_eq!(out.len(), MAX);
    assert_eq!(out.first().unwrap(), "50"); // oldest 50 dropped
}
