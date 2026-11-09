use super::*;

#[test]
fn the_stage_is_off_until_set_and_names_round_trip() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for s in Stage::ALL {
        assert_eq!(Stage::parse(s.name()), Some(s));
    }
    assert_eq!(Stage::parse("nope"), None);
    set_stage(Some(Stage::Orbit));
    assert_eq!(stage(), Some(Stage::Orbit));
    set_stage(None);
    assert_eq!(stage(), None);
}

#[test]
fn a_light_page_shows_less_of_it() {
    for s in Stage::ALL {
        assert!(strength(s, false) < strength(s, true));
        assert!(strength(s, true) < 1.0, "the panes' words read first");
    }
}

/// The scene is lit in the accent the app hands over, not the theme's
/// default; with none handed over it is the theme's. A changed key is a
/// changed palette, which is what `prepare` rebuilds the scene on.
#[test]
fn the_key_light_is_the_accent_handed_over() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let t = crew_theme::theme();
    set_key(None);
    assert_eq!(wanted_palette().key, t.accent_default);
    let red = (220, 30, 40);
    set_key(Some(red));
    assert_eq!(key(), Some(red));
    let p = wanted_palette();
    set_key(None);
    assert_eq!(p.key, red);
    assert_ne!(p, wanted_palette(), "a new accent rebuilds the scene");
}
