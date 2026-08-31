use super::*;

use std::sync::atomic::{AtomicU32, Ordering};
static SEQ: AtomicU32 = AtomicU32::new(0);

/// A fresh project dir per test — these run in parallel against a
/// process-wide filesystem. Mirrors `tests/common::unique_dir`.
fn tmp() -> PathBuf {
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("crew-spec-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn absent_store_loads_empty() {
    assert!(load_at(&tmp()).is_empty());
}

#[test]
fn corrupt_store_loads_empty_instead_of_panicking() {
    let base = tmp();
    std::fs::create_dir_all(base.join(".crew")).unwrap();
    std::fs::write(path(&base), "{not json").unwrap();
    assert!(load_at(&base).is_empty());
}

#[test]
fn record_then_load_roundtrips() {
    let base = tmp();
    record_at(&base, &[("archivist".into(), "records, retrieval".into())]);
    let got = load_at(&base);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].name, "archivist");
    assert_eq!(got[0].role, "records, retrieval");
    assert!(got[0].last_used > 0);
}

#[test]
fn record_merges_by_name_rather_than_suffixing() {
    let base = tmp();
    record_at(&base, &[("analyst".into(), "first".into())]);
    record_at(&base, &[("analyst".into(), "second".into())]);
    let got = load_at(&base);
    assert_eq!(got.len(), 1, "same name is the same specialist: {got:?}");
    assert_eq!(got[0].role, "first", "the original role is kept");
}

#[test]
fn record_skips_names_that_are_not_slugs() {
    let base = tmp();
    // "@#$" is `agentname::slug`'s own canonical unsalvageable example
    // (see `slug_or_derives_from_id_when_unsalvageable`): every char is
    // dropped, leaving nothing. A name with plain whitespace, like "Not A
    // Slug", is deliberately *salvageable* (hyphenated to "not-a-slug"),
    // so it wouldn't exercise this skip path.
    record_at(
        &base,
        &[("@#$".into(), "x".into()), ("ok-name".into(), "y".into())],
    );
    let got = load_at(&base);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].name, "ok-name");
}

#[test]
fn evicts_least_recently_used_at_cap() {
    let base = tmp();
    // Fill past the cap, oldest first.
    for i in 0..(CAP + 3) {
        record_at(&base, &[(format!("agent-{i:02}"), String::new())]);
    }
    let got = load_at(&base);
    assert_eq!(got.len(), CAP);
    let names: Vec<&str> = got.iter().map(|s| s.name.as_str()).collect();
    assert!(!names.contains(&"agent-00"), "oldest should be evicted");
    assert!(names.contains(&"agent-26"), "newest should survive");
}

#[test]
fn touch_defers_eviction_for_a_dialed_specialist() {
    // Without touch, last_used only moves when a run re-invents a name, so
    // a specialist you @-dial daily would be evicted by unrelated churn.
    let base = tmp();
    record_at(&base, &[("favourite".into(), String::new())]);
    for i in 0..(CAP - 1) {
        record_at(&base, &[(format!("filler-{i:02}"), String::new())]);
    }
    touch_at(&base, "favourite");
    // Two more push past the cap; `favourite` must outlive the fillers.
    record_at(&base, &[("newcomer-a".into(), String::new())]);
    record_at(&base, &[("newcomer-b".into(), String::new())]);
    let names: Vec<String> = load_at(&base).into_iter().map(|s| s.name).collect();
    assert!(names.contains(&"favourite".to_string()), "got {names:?}");
}

#[test]
fn touch_on_an_unknown_name_is_a_no_op() {
    let base = tmp();
    record_at(&base, &[("archivist".into(), String::new())]);
    touch_at(&base, "nobody");
    assert_eq!(load_at(&base).len(), 1);
}

#[test]
fn a_same_millisecond_tie_evicts_the_earliest_recorded() {
    // One call ⇒ every name shares a `last_used`, so only physical order
    // can break the tie. The newest write must never be the one evicted.
    let base = tmp();
    let names: Vec<(String, String)> = (0..(CAP + 2))
        .map(|i| (format!("agent-{i:02}"), String::new()))
        .collect();
    record_at(&base, &names);
    let got: Vec<String> = load_at(&base).into_iter().map(|s| s.name).collect();
    assert_eq!(got.len(), CAP);
    assert!(
        !got.contains(&"agent-00".to_string()),
        "earliest recorded evicted: {got:?}"
    );
    assert!(
        got.contains(&format!("agent-{:02}", CAP + 1)),
        "latest recorded survives: {got:?}"
    );
}
