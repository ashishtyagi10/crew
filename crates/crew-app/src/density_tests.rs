use super::*;

#[test]
fn every_level_round_trips_and_has_synonyms() {
    for d in Density::ALL {
        assert_eq!(Density::parse(d.as_str()), Some(d), "{}", d.as_str());
    }
    assert_eq!(Density::parse(" COMPACT "), Some(Density::Compact));
    assert_eq!(Density::parse("comfortable"), Some(Density::Roomy));
    assert_eq!(Density::parse("default"), Some(Density::Cozy));
    assert_eq!(Density::parse("airy"), None);
}

/// The global round-trips every level — a mis-mapped discriminant would
/// silently pin the whole canvas to one density, and the `_ =>` arm makes
/// that a live risk rather than a theoretical one.
#[test]
fn the_global_round_trips_every_level() {
    let _g = crate::app::motion_test_guard();
    for d in Density::ALL {
        set_level(d);
        assert_eq!(level(), d);
        assert_eq!(gap(), d.gap_px());
    }
    set_level(Density::Cozy);
}

/// A ladder that does not actually step is the failure this whole feature
/// would ship as: three names, one layout.
#[test]
fn the_ladder_is_strictly_ordered_in_both_axes() {
    let g: Vec<f32> = Density::ALL.iter().map(|d| d.gap_px()).collect();
    let r: Vec<usize> = Density::ALL.iter().map(|d| d.card_gap_rows()).collect();
    assert!(g[0] < g[1] && g[1] < g[2], "{g:?}");
    assert!(r[0] < r[1] && r[1] < r[2], "{r:?}");
}

/// Cozy must be exactly what crew drew before the knob existed, or every
/// existing user's canvas shifts on upgrade for no reason they asked for.
#[test]
fn cozy_is_the_layout_crew_already_had() {
    assert_eq!(Density::Cozy.gap_px(), 8.0);
    assert_eq!(Density::Cozy.card_gap_rows(), 1);
}

/// Compact closes the chat spacer but never the pane gutter — two cards
/// whose strokes touch read as one card with a seam.
#[test]
fn compact_still_leaves_a_gutter_between_cards() {
    assert!(Density::Compact.gap_px() > 0.0);
    assert_eq!(Density::Compact.card_gap_rows(), 0);
}
