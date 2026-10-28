use crate::*;

const CLASSICS: [(&str, ThemeId); 5] = [
    ("tokyo-night", ThemeId::TokyoNight),
    ("solarized", ThemeId::Solarized),
    ("everforest", ThemeId::Everforest),
    ("gruvbox", ThemeId::Gruvbox),
    ("monokai", ThemeId::Monokai),
];

/// `classic` names the rotation and each palette's own name pins it, so
/// `/theme gruvbox` holds Gruvbox and `/theme classic` walks all five.
#[test]
fn each_classic_pins_by_name_and_the_look_rotates_them() {
    let _g = crate::test_guard();
    assert_eq!(
        parse_selection("classic"),
        Some(Selection::Mode(RandomMode::Classic))
    );
    for (name, id) in CLASSICS {
        assert_eq!(parse_selection(name), Some(Selection::Fixed(id)), "{name}");
        assert_eq!(id.as_str(), name);
        assert_eq!(id.mode(), RandomMode::Classic, "{name}");
        assert!(pinnable_themes().any(|t| t == id), "{name} not offered");
    }
    apply_selection(Selection::Mode(RandomMode::Classic), 1_000);
    let mut seen = vec![current_id()];
    for i in 1..=40u64 {
        tick_random(1_000 + i * ROTATE_MS);
        assert_eq!(current_id().mode(), RandomMode::Classic, "left classic");
        if !seen.contains(&current_id()) {
            seen.push(current_id());
        }
    }
    assert!(seen.len() >= 4, "classic only ever showed {seen:?}");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// Every classic is dark smoked glass with white words, like `nebula`: the
/// white-text glass's shade, a ring, and no tube.
#[test]
fn every_classic_is_white_text_glass() {
    for (name, id) in CLASSICS {
        let t = id.theme();
        assert!(id.is_clear_glass(), "{name}");
        assert!(!t.is_tube() && !t.is_print(), "{name}");
        assert!(t.modern.is_some(), "{name}: no ring");
    }
}
