use super::{fitting, forms, hint};

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

/// A card clamped by a narrow pane takes a shorter WHOLE hint rather than a
/// cut one. The sign-in flow's only sign of life used to read `waiting for
/// browser · or pa…`.
#[test]
fn a_narrow_card_gets_a_shorter_hint_not_a_cut_one() {
    let long = fitting("", true, 60).expect("a hint");
    assert_eq!(long, "waiting for browser \u{b7} or paste the key");
    let short = fitting("", true, 24).expect("a hint");
    assert_eq!(short, "waiting for browser");
    for inner in 1..60usize {
        let f = fitting("NVIDIA_API_KEY", false, inner).expect("a hint");
        assert!(!f.ends_with('\u{2026}') || f == "waiting\u{2026}", "{f:?}");
        // Either it fits, or it is the shortest there is — never a middle
        // form that does not fit.
        let shortest = forms("NVIDIA_API_KEY", false).last().copied().unwrap();
        assert!(
            crate::chatwidth::str_w(f) <= inner || f == shortest,
            "{inner}: {f:?}"
        );
    }
}

/// Every ladder is written longest-first, which is what makes "the first that
/// fits" the best that fits.
#[test]
fn every_ladder_is_longest_first() {
    for (var, waiting) in [("NVIDIA_API_KEY", false), ("", true)] {
        let widths: Vec<usize> = forms(var, waiting)
            .iter()
            .map(|f| crate::chatwidth::str_w(f))
            .collect();
        assert!(
            widths.windows(2).all(|w| w[0] > w[1]),
            "{var} {waiting}: {widths:?}"
        );
    }
}
