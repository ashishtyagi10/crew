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
