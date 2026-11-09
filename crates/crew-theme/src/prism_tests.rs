use crate::*;

/// `prism` names both the look and its one page, so either way of saying
/// it serves the page, the picker lists it once, and it holds through a
/// rotation tick.
#[test]
fn prism_names_the_one_page() {
    let _g = crate::test_guard();
    assert_eq!(
        parse_selection("prism"),
        Some(Selection::Mode(RandomMode::Prism))
    );
    assert_eq!(ThemeId::from_name("prism"), Some(ThemeId::Prism));
    assert_eq!(RandomMode::Prism.sole_palette(), Some(ThemeId::Prism));
    assert!(!pinnable_themes().any(|t| t == ThemeId::Prism));
    apply_selection(Selection::Mode(RandomMode::Prism), 1_000);
    assert_eq!(current_id(), ThemeId::Prism);
    tick_random(1_000 + ROTATE_MS);
    assert_eq!(current_id(), ThemeId::Prism, "prism left its page");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// Prism is white-text glass on a jade page (green the largest channel,
/// blue close behind, red far under both), sheerer in the slab than the
/// smoked glass so its scene reads through a pane.
#[test]
fn prism_is_white_text_glass_on_a_jade_page() {
    let id = ThemeId::Prism;
    let t = id.theme();
    assert!(id.is_clear_glass());
    assert!(!t.is_tube() && !t.is_print());
    assert!(t.modern.is_some(), "no ring");
    let (r, g, b) = t.page_bg;
    assert!(g > b && b > r + 15, "page {:?} is not jade", t.page_bg);
    let body = |id: ThemeId| id.theme().liquid.map(|l| l.body).unwrap_or(1.0);
    assert!(
        body(id) < body(ThemeId::Amethyst),
        "the slab hides the scene"
    );
}

/// Prism is the one theme with a scene of its own; every other palette
/// draws none unless the user picks one, so adding it changed no one else.
#[test]
fn only_prism_brings_a_scene() {
    for id in ALL_THEMES {
        let want = (id == ThemeId::Prism).then_some("prism");
        assert_eq!(id.scene(), want, "{}", id.as_str());
    }
}
