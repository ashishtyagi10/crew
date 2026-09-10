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

/// The prompt asks for room for a typical key from the start, for its hint
/// if that is longer, and for every character typed — it grows with the key.
#[test]
fn the_prompt_wants_room_for_a_key_its_hint_and_what_was_typed() {
    use super::want;
    // Two columns for the `❯ ` prompt at the head of the field, then…
    assert_eq!(want(None, 0), 2 + 48, "a typical key's columns");
    assert_eq!(
        want(Some(&"h".repeat(40)), 0),
        2 + 48,
        "a short hint does not shrink it"
    );
    assert_eq!(
        want(Some(&"h".repeat(60)), 10),
        2 + 60,
        "a long hint widens it"
    );
    assert_eq!(want(None, 100), 2 + 100, "a long paste widens it");
}
