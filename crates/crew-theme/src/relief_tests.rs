use crate::*;

const RELIEF: [ThemeId; 4] = [
    ThemeId::Porcelain,
    ThemeId::Sand,
    ThemeId::Mist,
    ThemeId::Slate,
];

/// `soft` (neumorphism) is a look of four palettes: its name rotates among them, each
/// pins itself by name, and the picker lists all four.
#[test]
fn relief_is_a_look_of_four() {
    let _g = crate::test_guard();
    for name in ["soft", "neumorphism", "neumorph", "relief"] {
        assert_eq!(
            parse_selection(name),
            Some(Selection::Mode(RandomMode::Relief)),
            "{name}"
        );
    }
    assert_eq!(RandomMode::Relief.sole_palette(), None);
    for id in RELIEF {
        assert_eq!(id.mode(), RandomMode::Relief);
        assert_eq!(ThemeId::from_name(id.as_str()), Some(id));
        assert_eq!(parse_selection(id.as_str()), Some(Selection::Fixed(id)));
        assert!(pinnable_themes().any(|t| t == id), "{}", id.as_str());
    }
    apply_selection(Selection::Mode(RandomMode::Relief), 1_000);
    assert!(RELIEF.contains(&current_id()));
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// Only these four stand in relief, and each page leaves both of a card's
/// shadows room: something lighter to throw, and something darker.
#[test]
fn only_relief_palettes_stand_in_relief_with_room_for_both_shadows() {
    for id in ALL_THEMES {
        let t = id.theme();
        assert_eq!(t.relief() > 0.0, RELIEF.contains(&id), "{}", id.as_str());
        if t.relief() == 0.0 {
            continue;
        }
        let g = glass_style_for(t);
        let above = contrast_ratio(g.highlight, t.page_bg);
        let below = contrast_ratio(t.page_bg, (0, 0, 0));
        assert!(above >= 1.15, "{}: light shadow {above:.2}", id.as_str());
        assert!(below >= 1.2, "{}: dark shadow {below:.2}", id.as_str());
    }
}

/// A card is a SOLID slab of the page's own colour, so it is told from the
/// page by its shadows alone and the words on it stand on exactly the field
/// every role was derived against; the dark shadow is black, not a halo.
#[test]
fn a_relief_card_is_a_solid_slab_with_a_shadow() {
    for id in RELIEF {
        let t = id.theme();
        let g = glass_style_for(t);
        assert_eq!(g.tint, t.page_bg, "{}", id.as_str());
        assert_eq!((g.alpha_top, g.alpha_bottom), (1.0, 1.0));
        assert!(g.shadow_alpha >= 0.25 && g.glow == 0.0, "{}", id.as_str());
        assert!(g.highlight_alpha > 0.0, "{}: no light shadow", id.as_str());
        assert_eq!(t.liquid.map(|l| l.body), Some(1.0));
    }
}
