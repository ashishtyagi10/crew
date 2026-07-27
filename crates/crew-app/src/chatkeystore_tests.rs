use super::*;

/// Read back the JSON store at `path` (empty `Store` on any failure — same
/// as `credentials::load_from`, reimplemented here rather than exposed
/// publicly, since this is the only test file that needs it).
fn load(path: &Path) -> crew_plugin::credentials::Store {
    crew_plugin::credentials::load_from(path)
}

#[test]
fn submit_writes_the_key_to_the_injected_path_and_notes_the_var_and_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    let mut pane = crate::chat::tests::pane();

    store_provider_key_at(
        &mut pane,
        &path,
        "ANTHROPIC_API_KEY",
        "sk-test-not-a-real-key",
    );

    let store = load(&path);
    assert_eq!(
        store.keys.get("ANTHROPIC_API_KEY").map(String::as_str),
        Some("sk-test-not-a-real-key"),
        "the key must land at the injected path, not the real credentials file"
    );
    assert_eq!(store.provider.as_deref(), Some("anthropic"));

    let note = pane
        .messages
        .last()
        .expect("a note was pushed")
        .text
        .clone();
    assert!(
        note.contains("ANTHROPIC_API_KEY"),
        "note names the variable: {note:?}"
    );
    assert!(
        note.contains("anthropic"),
        "note names the pinned provider: {note:?}"
    );
    assert!(
        !note.contains("sk-test-not-a-real-key"),
        "note must never contain the secret value: {note:?}"
    );
}

#[test]
fn submit_failure_names_the_variable_and_never_the_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    let mut pane = crate::chat::tests::pane();

    // Not in `credentials::VARS`, so `save_key_at` bails before writing
    // anything — this is the failure path, exercised without touching disk.
    store_provider_key_at(
        &mut pane,
        &path,
        "NOT_A_REAL_PROVIDER_VAR",
        "sk-should-never-appear",
    );

    assert!(!path.exists(), "a rejected var must never be written");
    let note = pane
        .messages
        .last()
        .expect("a note was pushed")
        .text
        .clone();
    assert!(
        note.contains("NOT_A_REAL_PROVIDER_VAR"),
        "failure note names the variable: {note:?}"
    );
    assert!(
        !note.contains("sk-should-never-appear"),
        "failure note must never contain the secret value: {note:?}"
    );
}
