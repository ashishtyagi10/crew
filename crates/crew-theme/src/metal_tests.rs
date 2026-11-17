use crate::*;

/// `metal` names the look and `mercury` is its one page, so either name
/// (and the spelled-out ones) serves it, and the picker lists it once, as
/// `metal`.
#[test]
fn metal_and_mercury_both_serve_the_one_page() {
    let _g = crate::test_guard();
    for name in [
        "metal",
        "liquid-metal",
        "liquidmetal",
        "liquid metal",
        "chrome",
        "mercury",
        "METAL",
    ] {
        assert_eq!(
            parse_selection(name),
            Some(Selection::Mode(RandomMode::Metal)),
            "{name}"
        );
    }
    assert_eq!(ThemeId::Mercury.as_str(), "mercury");
    assert_eq!(RandomMode::Metal.as_str(), "metal");
    assert_eq!(RandomMode::Metal.sole_palette(), Some(ThemeId::Mercury));
    assert!(!pinnable_themes().any(|t| t == ThemeId::Mercury));
    apply_selection(Selection::Mode(RandomMode::Metal), 1_000);
    assert_eq!(current_id(), ThemeId::Mercury);
    tick_random(1_000 + ROTATE_MS);
    assert_eq!(current_id(), ThemeId::Mercury, "metal left its page");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// Mercury is white-text glass with a ring and a scene of its own behind it,
/// on a gunmetal page: nearly grey, blue a hair above red. Its accent is
/// steel, a grey with only a cold cast, the metal itself.
#[test]
fn mercury_is_gunmetal_glass_with_a_steel_accent() {
    let id = ThemeId::Mercury;
    let t = id.theme();
    assert!(id.is_clear_glass());
    assert!(!t.is_tube() && !t.is_print());
    assert!(t.modern.is_some(), "no ring");
    assert_eq!(id.scene(), Some("mercury"));
    let (r, g, b) = t.page_bg;
    assert!(
        b >= g && g >= r && b - r <= 8,
        "page {:?} is not gunmetal",
        t.page_bg
    );
    let a = oklch::from_srgb(t.accent_default);
    assert!(
        a.c < 0.04,
        "accent {:?} is a colour, not steel",
        t.accent_default
    );
}
