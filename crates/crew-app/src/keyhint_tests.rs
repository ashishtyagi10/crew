use super::hint;

/// A key nobody hands out for free gets no hint row; the browser flow's hint
/// outranks a variable's own.
#[test]
fn only_free_keys_and_live_sign_ins_get_a_hint_row() {
    assert_eq!(hint("ANTHROPIC_API_KEY", false), None);
    assert!(hint("NVIDIA_API_KEY", false).is_some_and(|h| h.contains("build.nvidia.com")));
    assert!(hint("NVIDIA_API_KEY", true).is_some_and(|h| h.contains("waiting for browser")));
    assert!(hint("OPENROUTER_API_KEY", true).is_some_and(|h| h.contains("waiting for browser")));
}

/// Every hint must fit a modest card, and every variable the store can hold
/// must resolve here without panicking.
#[test]
fn hints_are_short_and_every_storable_variable_resolves() {
    for var in crew_plugin::credentials::VARS {
        for waiting in [false, true] {
            if let Some(h) = hint(var, waiting) {
                assert!(h.chars().count() <= 48, "{var}: hint too long: {h}");
            }
        }
    }
}
