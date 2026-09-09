use super::*;

/// nf-dev-rust: the Nerd Font probe glyph crew-app uses to decide whether
/// the icon set is on. Private Use Area — no ordinary text face maps it.
const PUA: char = '\u{e7a8}';

#[test]
fn basic_latin_is_covered_by_the_embedded_face() {
    assert!(has_glyph(None, 'a'));
    assert!(has_glyph(Some(crew_theme::EMBEDDED_FAMILY), 'a'));
    assert!(has_glyph(None, '0'));
}

#[test]
fn a_pua_icon_is_not_in_the_embedded_face() {
    // Asserted against the face crew ships, so the answer does not depend on
    // what this machine has installed.
    assert!(!has_glyph(None, PUA));
    assert!(!has_glyph(Some(crew_theme::EMBEDDED_FAMILY), PUA));
}

#[test]
fn a_family_that_is_not_installed_covers_nothing() {
    assert!(!has_glyph(Some("No Such Family 9f3e"), 'a'));
}

#[test]
fn the_answer_is_memoised_per_family_and_char() {
    assert!(!memoised("No Such Family 4b1c", 'q'));
    let _ = has_glyph(Some("No Such Family 4b1c"), 'q');
    assert!(memoised("No Such Family 4b1c", 'q'));
    // `None` memoises under the embedded family's own name.
    let _ = has_glyph(None, 'z');
    assert!(memoised(crew_theme::EMBEDDED_FAMILY, 'z'));
}
