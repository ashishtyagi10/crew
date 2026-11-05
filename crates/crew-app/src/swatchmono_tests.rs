use super::*;
use crate::swatch::for_value;
use crew_theme::ALL_THEMES;

/// On either Ink page, `dark` and `light` each show a page square and an ink
/// square, both bare (no capsule, so no hollow box), both visible on the page
/// under the picker, and the two modes do not draw the same strip. Today's
/// one capsuled accent chip fails every clause but the last.
#[test]
fn an_ink_page_shows_its_page_and_its_words_as_bare_squares() {
    let _g = crate::app::theme_test_guard();
    for under in [ThemeId::PaperLight, ThemeId::PaperDark] {
        crew_theme::set_theme(under);
        let page = crew_theme::theme().page_bg;
        let (dark, light) = (for_value("/theme", "dark"), for_value("/theme", "light"));
        for (name, strip) in [("dark", &dark), ("light", &light)] {
            assert_eq!(strip.len(), 2, "{name} on {}", under.as_str());
            for c in strip.iter() {
                assert_eq!(c.bg, None, "{name} on {}: a capsule", under.as_str());
                let cr = crew_theme::contrast_ratio(c.fg, page);
                assert!(cr >= MARK, "{name} on {}: {cr:.2}", under.as_str());
            }
        }
        assert_ne!(dark, light, "on {}", under.as_str());
    }
    crew_theme::set_theme(ThemeId::PaperDark);
}

/// The page comes first and it is the darker square for `dark`, the lighter
/// for `light`: the strip reads in the order the descriptions do.
#[test]
fn the_page_square_comes_before_the_ink_square() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::GlassNight);
    let lum = |c: &Chip| u32::from(c.fg.0) + u32::from(c.fg.1) + u32::from(c.fg.2);
    let dark = chips(ThemeId::PaperDark).unwrap();
    let light = chips(ThemeId::PaperLight).unwrap();
    assert!(lum(&dark[0]) < lum(&dark[1]));
    assert!(lum(&light[0]) > lum(&light[1]));
    crew_theme::set_theme(ThemeId::PaperDark);
}

/// Only a palette with no hue trades its accent chip: one with a colour of
/// its own is still told apart by it.
#[test]
fn a_palette_with_a_hue_keeps_its_accent_chip() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let ink_page = matches!(id, ThemeId::PaperDark | ThemeId::PaperLight);
        assert_eq!(chips(id).is_some(), ink_page, "{}", id.as_str());
    }
}

/// In `auto`, a pool of many, an Ink page is one cell, its page: the strip
/// stays one chip a palette, so the widest swatch column does not grow.
#[test]
fn in_a_pool_of_many_an_ink_page_is_its_page_alone() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::PaperLight);
    let auto = for_value("/theme", "auto");
    assert_eq!(auto.len(), ALL_THEMES.len());
    assert_eq!(auto[0], chips(ThemeId::PaperDark).unwrap()[0]);
    assert!(auto
        .iter()
        .all(|c| c.bg.is_none() || c.fg != (242, 242, 242)));
    crew_theme::set_theme(ThemeId::PaperDark);
}
