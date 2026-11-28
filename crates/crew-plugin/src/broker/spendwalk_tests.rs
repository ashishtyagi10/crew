//! A walk over the source: a NEW place a task builds a provider fails until
//! it chooses to be held at the cap or exempt, with why.

/// Every non-test file under `broker` that builds a provider for a call,
/// with what it does about the cap: a marker its source must hold, or a
/// reason it is exempt. A file not listed fails, so a new place a task
/// spends is decided on purpose, not found on an overnight bill.
#[test]
fn every_place_a_task_builds_a_provider_chooses_capped_or_exempt() {
    const CAPPED: &[(&str, &str)] = &[
        ("hopturn.rs", "spendcapwrap::wrap("),
        ("swarmconf.rs", "spendcapwrap::wrap_swarm("),
        ("swarmanswer.rs", "Gate::here("),
        ("fan.rs", "Gate::here("),
        ("swarmverify.rs", "live_judge_call("),
        ("replyverify.rs", "live_judge_call("),
        ("intent/classify.rs", "Gate::here"),
    ];
    const EXEMPT: &[(&str, &str)] = &[
        (
            "compact.rs",
            "cheap summary of an overflowing log, on the event path",
        ),
        (
            "sysexplore.rs",
            "read-only sub-agent with its own limits and bill",
        ),
        (
            "ask.rs",
            "the GUI's one-shot asks (far, explain): no task thread, no meter",
        ),
        (
            "advise.rs",
            "the advisor: a second opinion, billed to the thread already",
        ),
        ("prewarm.rs", "warms a connection, spends nothing"),
        ("discover.rs", "where providers are built, not a call"),
        ("intent/mod.rs", "re-export"),
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/broker");
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = e.path();
            let rel = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            if path.is_dir() {
                if !rel.contains("tests") {
                    stack.push(path);
                }
                continue;
            }
            if !rel.ends_with(".rs") || rel.contains("_tests") || rel.contains("tests/") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let builds = [
                "provider_and_model_for(",
                "live_provider_at(",
                "ApiFactory::new(",
            ]
            .iter()
            .any(|m| {
                src.lines()
                    .any(|l| l.contains(m) && !l.trim_start().starts_with("//"))
            });
            if !builds && !CAPPED.iter().any(|(f, _)| *f == rel) {
                continue;
            }
            if let Some((_, marker)) = CAPPED.iter().find(|(f, _)| *f == rel) {
                assert!(src.contains(marker), "{rel} lost its cap: `{marker}`");
            } else {
                assert!(
                    EXEMPT.iter().any(|(f, _)| *f == rel),
                    "{rel} builds a provider: hold it at the cap (`spendcapwrap`) or list it exempt, with why"
                );
            }
        }
    }
}
