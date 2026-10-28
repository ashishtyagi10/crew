use crate::*;

/// `purple` names the look and `amethyst` is its one page, so either name
/// serves it, the way `neon` and `neon-dark` do, and the picker lists it
/// once, as `purple`.
#[test]
fn purple_and_amethyst_both_serve_the_one_page() {
    let _g = crate::test_guard();
    for name in ["purple", "amethyst"] {
        assert_eq!(
            parse_selection(name),
            Some(Selection::Mode(RandomMode::Purple)),
            "{name}"
        );
    }
    assert_eq!(ThemeId::Amethyst.as_str(), "amethyst");
    assert_eq!(RandomMode::Purple.sole_palette(), Some(ThemeId::Amethyst));
    assert!(!pinnable_themes().any(|t| t == ThemeId::Amethyst));
    apply_selection(Selection::Mode(RandomMode::Purple), 1_000);
    assert_eq!(current_id(), ThemeId::Amethyst);
    tick_random(1_000 + ROTATE_MS);
    assert_eq!(current_id(), ThemeId::Amethyst, "purple left its page");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// Amethyst is dark smoked glass with white words and a ring, and its page
/// is purple: red and blue both well above green, blue the larger.
#[test]
fn amethyst_is_white_text_glass_on_a_purple_page() {
    let id = ThemeId::Amethyst;
    let t = id.theme();
    assert!(id.is_clear_glass());
    assert!(!t.is_tube() && !t.is_print());
    assert!(t.modern.is_some(), "no ring");
    let (r, g, b) = t.page_bg;
    assert!(b > r && r > g + 20, "page {:?} is not purple", t.page_bg);
}
