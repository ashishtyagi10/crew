use super::*;
use crate::broker::{specialists, testenv};
use crate::{CliAdapter, Normalize};

fn reg(names: &[&str]) -> Registry {
    Registry::new(
        names
            .iter()
            .map(|n| {
                Box::new(CliAdapter {
                    name: (*n).into(),
                    program: "true".into(),
                    args: vec![],
                    normalize: Normalize::Raw,
                }) as Box<dyn crate::Adapter>
            })
            .collect(),
    )
}

#[test]
fn roster_lists_agents_when_present() {
    // A non-empty roster short-circuits before the provider check, so this is
    // env-independent (no guard needed).
    assert!(roster(&reg(&["claude", "codex"])).contains("claude, codex"));
}

#[test]
fn empty_roster_without_a_provider_says_set_a_key() {
    // no_provider() clears every key + CREW_PROVIDER for the guard's lifetime,
    // deterministic even on this machine (which exports DASHSCOPE_API_KEY).
    let _env = testenv::no_provider();
    assert!(roster(&reg(&[])).contains("ANTHROPIC_API_KEY"));
}

#[test]
fn empty_roster_with_a_provider_invites_a_task_not_a_key() {
    // The fresh-project state: a working provider (mock resolves like any key)
    // but an empty specialist store — the store fills on the first run. The
    // greeting must NOT wrongly blame a missing key; that was the reported bug.
    let _env = testenv::mock("hi");
    let m = roster(&reg(&[]));
    assert!(
        !m.contains("ANTHROPIC_API_KEY"),
        "must not tell a user with a working provider to set a key: {m}"
    );
    assert!(
        m.to_lowercase().contains("type a task"),
        "should invite a task: {m}"
    );
}

/// A fresh project dir per test, matching `specialists.rs`'s own `tmp()` —
/// `testenv::mock`'s dir is private to `mod.rs`, and this test needs to
/// hand-craft the store's on-disk order (`mock_with_specialists` stamps every
/// seeded entry with the SAME instant, which can't express "`favourite` is
/// the single oldest entry").
fn project_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("crew-stdio-{tag}-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Proves the WIRING of `specialists::touch` on the `@`-dial path (`relay.rs`
/// `dialed_target` → `stdio.rs` `relay_counting`), not just `touch_at`'s own
/// LRU semantics (already proven in `specialists.rs`). Fills the store to
/// exactly `CAP`, oldest-first, with `favourite` the very oldest entry —
/// `record_at`'s move-to-front-on-insert means physical order alone encodes
/// recency even under millisecond timestamp ties (the same technique
/// `specialists.rs`'s `evicts_least_recently_used_at_cap` relies on). Address
/// `favourite` via `relay_counting` (the real `@`-dial entry point, not a
/// direct `touch_at` call), then invent two more specialists — enough to push
/// the store past `CAP` twice over. Without the wiring, `favourite` is still
/// the oldest and is the first evicted; with it, addressing moved `favourite`
/// to the front first, so it must survive.
#[test]
fn at_dial_of_an_existing_specialist_defers_its_eviction() {
    let _g = testenv::mock("ok\n@done"); // holds the CREW_PROJECT_DIR/MOCK_REPLY lock
    let base = project_dir("touch-wiring");
    std::env::set_var("CREW_PROJECT_DIR", &base);

    specialists::record_at(&base, &[("favourite".into(), String::new())]);
    for i in 0..(specialists::CAP - 1) {
        specialists::record_at(&base, &[(format!("filler-{i:02}"), String::new())]);
    }
    assert_eq!(specialists::load_at(&base).len(), specialists::CAP);

    let session = Session::new();
    let mut evs = Vec::new();
    relay_counting(
        "@favourite do the thing",
        &session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    // Sanity: the dial actually reached `favourite`, not the roster's
    // default first agent (registration order isn't guaranteed here).
    assert!(
        evs.iter().any(|e| matches!(
            e,
            PluginEvent::Message { sender, .. } if sender.starts_with("favourite")
        )),
        "expected favourite to have been dialed: {evs:?}"
    );

    // Two more runs invent new specialists, pushing the store past CAP twice.
    specialists::record_at(&base, &[("newcomer-a".into(), String::new())]);
    specialists::record_at(&base, &[("newcomer-b".into(), String::new())]);

    let names: Vec<String> = specialists::load_at(&base)
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert!(
        names.contains(&"favourite".to_string()),
        "addressing favourite must have deferred its eviction: {names:?}"
    );
}
