use super::*;

#[test]
fn every_level_round_trips() {
    for l in GradientLevel::ALL {
        assert_eq!(GradientLevel::parse(l.as_str()), Some(l));
    }
    assert_eq!(
        GradientLevel::parse(" LIVELY "),
        Some(GradientLevel::Lively)
    );
    assert_eq!(GradientLevel::parse("fixed"), Some(GradientLevel::Off));
    assert_eq!(GradientLevel::parse("rainbow"), None);
}

/// Off is a genuine off: no lean at all, so the theme's poles are its own
/// bytes and idle frames stay identical.
#[test]
fn off_is_a_genuine_off() {
    assert_eq!(GradientLevel::Off.span_deg(), 0.0);
}

/// The ladder climbs, and never past what the theme layer will store.
#[test]
fn the_ladder_climbs_and_stays_in_range() {
    let _g = crate::app::theme_test_guard();
    let (o, s, l) = (
        GradientLevel::Off.span_deg(),
        GradientLevel::Subtle.span_deg(),
        GradientLevel::Lively.span_deg(),
    );
    assert!(o < s && s < l, "{o} {s} {l}");
    assert!(l <= crew_theme::poleshift::MAX_SHIFT_DEG, "{l}");
}

#[test]
fn the_global_round_trips_every_level() {
    let prev = level();
    for l in GradientLevel::ALL {
        set_level(l);
        assert_eq!(level(), l);
    }
    set_level(prev);
}
