use super::*;

/// One definition of the bands, shared with the fill colour — the mark
/// and the colour must never disagree about which band a reading is in.
#[test]
fn the_tiers_match_the_colour_thresholds() {
    assert_eq!(Tier::of(0.0), Tier::Nominal);
    assert_eq!(Tier::of(0.69), Tier::Nominal);
    assert_eq!(Tier::of(0.7), Tier::Warn);
    assert_eq!(Tier::of(0.89), Tier::Warn);
    assert_eq!(Tier::of(0.9), Tier::Critical);
    assert_eq!(Tier::of(1.0), Tier::Critical);
}

/// The cues appear only when asked for, and then they must actually
/// distinguish — three tiers that all mark the same are no better than
/// three tiers that all look the same.
#[test]
fn the_marks_appear_only_when_asked_and_tell_the_tiers_apart() {
    let _g = crate::app::motion_test_guard();
    set(false);
    for t in [Tier::Nominal, Tier::Warn, Tier::Critical] {
        assert_eq!(t.mark(), None, "{t:?} marked with the cues off");
    }
    set(true);
    assert_eq!(Tier::Nominal.mark(), None, "nominal is the quiet case");
    let (w, c) = (Tier::Warn.mark(), Tier::Critical.mark());
    assert!(w.is_some() && c.is_some());
    assert_ne!(w, c, "warning and critical must not share a mark");
    set(false);
}

/// Busy and merely-recent were both a solid dot, separated by a pulse —
/// brightness, which is the channel this whole module exists to stop
/// relying on.
#[test]
fn a_working_pane_gets_its_own_glyph_only_when_asked() {
    let _g = crate::app::motion_test_guard();
    set(false);
    assert_eq!(dot(true), dot(false), "off, both are the solid dot");
    set(true);
    assert_ne!(dot(true), dot(false), "on, working must look different");
    assert_eq!(dot(false), '\u{25cf}', "the quiet case never changes");
    set(false);
}
