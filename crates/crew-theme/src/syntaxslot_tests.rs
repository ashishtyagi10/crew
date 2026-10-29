//! Each palette's syntax map is a real choice, not a copy of the house's.
use super::*;
use crate::{ThemeId, ALL_THEMES};

/// Every palette with a map of its own: the classics, neon, amethyst,
/// blossom and the Ink Reader pages.
const OWN_MAP: [ThemeId; 10] = [
    ThemeId::TokyoNight,
    ThemeId::Solarized,
    ThemeId::Everforest,
    ThemeId::Gruvbox,
    ThemeId::Monokai,
    ThemeId::NeonDark,
    ThemeId::Amethyst,
    ThemeId::Blossom,
    ThemeId::PaperDark,
    ThemeId::PaperLight,
];

/// Every slot names a chromatic ANSI colour: 0 and 7 are the greys.
#[test]
fn every_class_takes_a_chromatic_slot() {
    for id in ALL_THEMES {
        let s = id.theme().syntax;
        for (what, slot) in s.classes().into_iter().chain([("code", s.code)]) {
            assert!((1..=6).contains(&slot), "{}: {what} = {slot}", id.as_str());
        }
    }
}

/// On a palette with a map of its own the five hued classes are five
/// different slots, which is how each scheme draws them, and the map is
/// really its own rather than the house's.
#[test]
fn an_own_map_gives_the_five_classes_five_slots() {
    for id in OWN_MAP {
        let s = id.theme().syntax;
        assert_ne!(s, SyntaxSlots::HOUSE, "{}", id.as_str());
        let c = s.classes();
        for i in 0..c.len() {
            for j in i + 1..c.len() {
                assert_ne!(c[i].1, c[j].1, "{}: {} == {}", id.as_str(), c[i].0, c[j].0);
            }
        }
    }
}

/// Every other palette keeps crew's mapping. A new palette that wants its
/// own has to be named in [`OWN_MAP`].
#[test]
fn every_other_palette_keeps_the_house_map() {
    for id in ALL_THEMES.into_iter().filter(|id| !OWN_MAP.contains(id)) {
        assert_eq!(id.theme().syntax, SyntaxSlots::HOUSE, "{}", id.as_str());
    }
}

/// The signatures the maps exist for: monokai's pink keyword and yellow
/// string, solarized's green statement, everforest's and gruvbox's red
/// keyword, Tokyo Night's purple one.
#[test]
fn each_classic_keyword_is_its_schemes() {
    let keyword = |id: ThemeId| id.theme().syntax.keyword;
    assert_eq!(keyword(ThemeId::Monokai), 1);
    assert_eq!(ThemeId::Monokai.theme().syntax.string, 3);
    assert_eq!(keyword(ThemeId::Solarized), 2);
    assert_eq!(keyword(ThemeId::Everforest), 1);
    assert_eq!(keyword(ThemeId::Gruvbox), 1);
    assert_eq!(keyword(ThemeId::TokyoNight), 5);
}

/// Neon's and amethyst's numbers are their hot red slot: a hot pink on
/// neon, a rose on amethyst.
#[test]
fn neon_and_amethyst_count_in_their_red() {
    for id in [ThemeId::NeonDark, ThemeId::Amethyst] {
        assert_eq!(id.theme().syntax.number, 1, "{}", id.as_str());
    }
}
