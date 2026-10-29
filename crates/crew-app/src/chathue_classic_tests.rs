//! A palette's own syntax map ([`crew_theme::SyntaxSlots`]) under the
//! floors: split from `chathue_tests` (line cap).
use super::super::*;

/// On a palette with a map of its own (the classics, neon, amethyst, blossom) the
/// five syntax classes read as five colours: each pair at least a visible
/// step apart in OKLab. A map is worth nothing if two of its classes land on
/// one ink once the floors have lifted them.
#[test]
fn own_map_classes_read_as_five_colours() {
    let _g = crate::app::theme_test_guard();
    let mut classics = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        if t.syntax == crew_theme::SyntaxSlots::HOUSE {
            continue;
        }
        classics += 1;
        let (base, h) = (chatink::derive(t), derive(t));
        let five = [
            ("keyword", h.keyword),
            ("type", h.ty),
            ("func", h.func),
            ("number", h.number),
            ("string", base.string),
        ];
        for i in 0..five.len() {
            for j in i + 1..five.len() {
                let d = crew_theme::oklch::distance(five[i].1, five[j].1);
                assert!(
                    d >= 0.05,
                    "{}: {} vs {} only {d:.3} apart",
                    id.as_str(),
                    five[i].0,
                    five[j].0
                );
            }
        }
    }
    assert_eq!(classics, 8, "the classics, neon, amethyst and blossom");
}

/// A class in a slot of its own never lands on the code colour, on any
/// palette: if it does, its slot could not clear the floors and fell back.
/// A class the map puts in the code's slot IS the code colour, by design.
#[test]
fn no_class_falls_back_to_code() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES.into_iter().filter(|id| !id.is_crt()) {
        let t = id.theme();
        let (base, h, s) = (chatink::derive(t), derive(t), t.syntax);
        for (what, c, slot) in [
            ("keyword", h.keyword, s.keyword),
            ("type", h.ty, s.ty),
            ("func", h.func, s.func),
            ("number", h.number, s.number),
        ] {
            match slot == s.code {
                true => assert_eq!(c, base.code, "{}: {what}", id.as_str()),
                false => assert_ne!(c, base.code, "{}: {what} fell back", id.as_str()),
            }
        }
    }
}
