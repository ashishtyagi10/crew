//! The hued classes answer to the same floors as the ladder — asserted per
//! preset per class, because "keyword reads on paper-dark" says nothing
//! about sepia-light, and the v0.6.34 palette shipped invisible on every
//! tube for exactly that reason.
use super::*;

fn hues(h: Hue) -> [(&'static str, Color); 4] {
    [
        ("keyword", h.keyword),
        ("type", h.ty),
        ("func", h.func),
        ("number", h.number),
    ]
}

/// Every hue, every preset: readable on the page, apart from body text,
/// readable on the code field. The attribute rides the comment rung and
/// answers to the comment's own (lower) page floor.
#[test]
fn every_hue_clears_the_three_floors_on_every_preset() {
    let _g = crate::app::theme_test_guard();
    let mut presets = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        let (base, h) = (chatink::derive(t), derive(t));
        for (what, c) in hues(h) {
            let (page, ink, field) = (
                contrast_ratio(c, t.page_bg),
                contrast_ratio(c, t.ink),
                contrast_ratio(c, base.code_bg),
            );
            assert!(
                clears_floors(c, t, base.code_bg),
                "{}: {what} page={page:.2} (need {PAGE_FLOOR}) ink={ink:.2} (need \
                 {SEPARATION_FLOOR}) field={field:.2} (need {CODE_ON_FIELD_FLOOR})",
                id.as_str(),
            );
        }
        let attr_page = contrast_ratio(h.attr, t.page_bg);
        assert!(
            attr_page >= 3.5,
            "{}: attr page={attr_page:.2}",
            id.as_str()
        );
        let attr_ink = contrast_ratio(h.attr, t.ink);
        assert!(
            attr_ink >= SEPARATION_FLOOR,
            "{}: attr ink={attr_ink:.2}",
            id.as_str()
        );
        presets += 1;
    }
    assert_eq!(presets, ALL_THEMES.len(), "every preset was checked");
}

/// On main the keyword slot WAS the code slot (`ansi[6]`, by design), so a
/// keyword and the identifier after it were one colour told apart by weight.
/// On every paper preset keyword, type and call now take three hues of their
/// own, and none of them is the code colour.
#[test]
fn paper_presets_draw_keyword_type_and_call_in_three_hues() {
    let _g = crate::app::theme_test_guard();
    let mut papers = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        if t.is_tube() {
            continue;
        }
        papers += 1;
        let (base, h) = (chatink::derive(t), derive(t));
        for (what, c) in [("keyword", h.keyword), ("type", h.ty), ("func", h.func)] {
            assert_ne!(c, base.code, "{}: {what} fell back to code", id.as_str());
        }
        assert_ne!(h.keyword, h.ty, "{}: keyword == type", id.as_str());
        assert_ne!(h.ty, h.func, "{}: type == func", id.as_str());
        assert_ne!(h.keyword, h.func, "{}: keyword == func", id.as_str());
    }
    assert_eq!(papers, 8, "every paper preset was checked");
}

/// The number takes `ansi[6]` — the code slot — through the same walk, so it
/// lands ON the code colour on every preset. Pinned so the choice is a
/// stated one: a number that read in a fifth hue would need a slot of its
/// own, and the ladder has none left that clears the floors on the tubes.
#[test]
fn a_number_shares_the_code_slot() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let t = id.theme();
        assert_eq!(derive(t).number, chatink::derive(t).code, "{}", id.as_str());
    }
}

/// A slot that cannot be lifted to the floors is not drawn faint: it falls
/// back to the code colour. Forced here with a mid-grey page, against which
/// nothing reaches 4.5:1.
#[test]
fn a_slot_that_cannot_clear_the_floors_falls_back_to_code() {
    let _g = crate::app::theme_test_guard();
    let mut t = *crew_theme::ThemeId::PaperDark.theme();
    t.page_bg = (128, 128, 128);
    let code = chatink::derive(&t).code;
    let h = derive(&t);
    for (what, c) in hues(h) {
        assert_eq!(c, code, "{what} did not fall back on an unliftable page");
    }
}

/// The live route: `chatink::token_fg` hands the hued tokens here, and a
/// keyword is no longer the code colour. On main
/// `token_fg(Keyword) == token_fg(Plain)` on every preset.
#[test]
fn token_fg_gives_a_keyword_a_colour_of_its_own() {
    let _g = crate::app::theme_test_guard();
    assert_ne!(
        chatink::token_fg(Token::Keyword),
        chatink::token_fg(Token::Plain),
        "keyword is still the code colour"
    );
    assert_eq!(chatink::token_fg(Token::Type), token_fg(Token::Type));
    assert_eq!(chatink::token_fg(Token::Func), token_fg(Token::Func));
    assert_eq!(chatink::token_fg(Token::Attr), token_fg(Token::Attr));
    // A ladder token asked of this module answers with code, not a panic.
    assert_eq!(token_fg(Token::Comment), chatink::code_fg());
}
