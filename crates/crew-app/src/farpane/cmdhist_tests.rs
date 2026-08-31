use super::*;

/// Point the home variables AND this store's path override at a fresh
/// tempdir for the duration of `f`, then restore them — locked
/// crate-wide via `envlock::with_vars`. The explicit path override is
/// what makes this isolation hold on Windows, where `dirs::config_dir()`
/// does not follow `$HOME`; see [`path`].
fn with_tmp_home<T>(f: impl FnOnce() -> T) -> T {
    let dir = tempfile::tempdir().unwrap();
    let mut vars: Vec<(&str, std::ffi::OsString)> = crate::envlock::HOME_VARS
        .iter()
        .map(|k| (*k, dir.path().into()))
        .collect();
    vars.push((
        "CREW_FAR_HISTORY_PATH",
        dir.path().join("far-history").into(),
    ));
    crate::envlock::with_vars(&vars, f)
}

#[test]
fn load_is_empty_when_no_file_exists() {
    with_tmp_home(|| {
        assert!(CmdHistory::load().entries.is_empty());
    });
}

#[test]
fn push_persists_and_reloads() {
    with_tmp_home(|| {
        let mut h = CmdHistory::load();
        h.push("ls");
        h.push("cargo test");
        let reloaded = CmdHistory::load();
        assert_eq!(
            reloaded.entries,
            vec!["ls".to_string(), "cargo test".to_string()]
        );
    });
}

#[test]
fn push_skips_blank_and_adjacent_duplicate() {
    with_tmp_home(|| {
        let mut h = CmdHistory::load();
        h.push("ls");
        h.push("ls"); // adjacent dupe, skipped
        h.push(""); // blank, skipped
        h.push("pwd");
        h.push("ls"); // not adjacent (pwd in between) — kept
        assert_eq!(
            h.entries,
            vec!["ls".to_string(), "pwd".to_string(), "ls".to_string()]
        );
    });
}

#[test]
fn push_caps_at_max_dropping_oldest() {
    with_tmp_home(|| {
        let mut h = CmdHistory::load();
        for i in 0..MAX + 10 {
            h.push(&format!("cmd{i}"));
        }
        assert_eq!(h.entries.len(), MAX);
        assert_eq!(h.entries.first().unwrap(), "cmd10"); // oldest 10 dropped
        assert_eq!(h.entries.last().unwrap(), &format!("cmd{}", MAX + 9));
    });
}

#[test]
fn prev_next_cycle_and_restore_typed_text() {
    let mut h = CmdHistory::from_entries(vec!["ls".into(), "pwd".into(), "cargo test".into()]);
    assert_eq!(h.prev("half-typed"), Some("cargo test")); // newest first
    assert_eq!(h.prev("half-typed"), Some("pwd"));
    assert_eq!(h.prev("half-typed"), Some("ls")); // oldest
    assert_eq!(h.prev("half-typed"), Some("ls")); // stays at oldest
    assert_eq!(h.next("ls"), Some("pwd"));
    assert_eq!(h.next("pwd"), Some("cargo test"));
    assert_eq!(h.next("cargo test"), Some("half-typed")); // restored
    assert_eq!(h.next("anything"), None); // not browsing anymore
}

#[test]
fn prev_with_no_history_returns_none() {
    let mut h = CmdHistory::from_entries(vec![]);
    assert_eq!(h.prev("typed"), None);
}

#[test]
fn ghost_matches_the_newest_extending_entry() {
    let h = CmdHistory::from_entries(vec![
        "cargo build".into(),
        "cargo check".into(),
        "cargo test".into(),
    ]);
    assert_eq!(h.ghost("cargo"), Some("cargo test")); // newest wins
    assert_eq!(h.ghost("cargo test"), None); // no STRICT extension
    assert_eq!(h.ghost("zz"), None); // no match
}

#[test]
fn ghost_is_none_on_an_empty_bar() {
    let h = CmdHistory::from_entries(vec!["cargo test".into()]);
    assert_eq!(h.ghost(""), None);
}
