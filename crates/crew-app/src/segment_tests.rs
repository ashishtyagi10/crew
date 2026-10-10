use super::*;
use crate::glyphs::force;
use crew_theme::contrast_ratio;
use unicode_width::UnicodeWidthChar;

/// No caps: a bare block.
const NONE: Caps = Caps {
    left: false,
    right: false,
    ground: None,
};

fn text(cells: &[Cell]) -> String {
    cells.iter().map(|c| c.c).collect()
}

/// Off the Nerd Font set the caps are half blocks; on it, the powerline
/// arcs — and either way each cap is one cell, in the block's colour, on
/// the ground behind it.
#[test]
fn caps_follow_the_glyph_switch_and_wear_the_block_colour() {
    let _g = crate::app::theme_test_guard();
    let (bg, page) = ((200, 40, 40), (10, 10, 10));
    let off = force(false);
    let b = badge("rust", page, bg, Caps::BOTH);
    assert_eq!(text(&b), "\u{2590} rust \u{258c}");
    drop(off);
    let _on = force(true);
    let b = badge("rust", page, bg, Caps::BOTH);
    assert_eq!(text(&b), "\u{e0b6} rust \u{e0b4}");
    for cap in [&b[0], &b[b.len() - 1]] {
        assert_eq!(cap.fg, bg, "a cap is drawn in the block's colour");
        assert_eq!(cap.bg, None, "…on the page");
        assert_eq!(UnicodeWidthChar::width(cap.c), Some(1), "one cell");
    }
    let on_field = badge("x", page, bg, Caps::on((30, 30, 30)));
    assert_eq!(on_field[0].bg, Some((30, 30, 30)), "a cap on a field");
    assert_eq!(text(&badge("x", page, bg, NONE)), " x ");
}

/// The label's ink clears the text floor against the block whatever was
/// asked for — the same colour as the block included.
#[test]
fn the_label_is_walked_to_the_text_floor() {
    let _g = crate::app::theme_test_guard();
    let bg = (120, 120, 120);
    let b = badge("hi", bg, bg, Caps::BOTH);
    let floor = crew_theme::contrast::text_floor();
    for cell in b.iter().filter(|c| c.bg == Some(bg)) {
        assert!(
            contrast_ratio(cell.fg, bg) >= floor,
            "{:?} on {bg:?} = {:.2} < {floor}",
            cell.fg,
            contrast_ratio(cell.fg, bg)
        );
    }
    // The label is bold; the pads and caps are not.
    assert!(b[2].bold && b[3].bold);
    assert!(!b[0].bold && !b[1].bold && !b[4].bold && !b[5].bold);
}

/// A badge is exactly as wide as [`width`] says: the label's display
/// columns plus a pad each side plus each cap — every cell one column,
/// the PUA caps included.
#[test]
fn a_badge_is_as_wide_as_its_text_plus_its_chrome() {
    let _g = crate::app::theme_test_guard();
    for on in [false, true] {
        let _f = force(on);
        for (label, caps) in [
            ("rust", Caps::BOTH),
            ("\u{65e5}\u{672c}", Caps::BOTH),
            ("a", NONE),
            ("", Caps::BOTH),
        ] {
            let b = badge(label, (0, 0, 0), (200, 200, 200), caps);
            let cols: usize = b.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
            assert_eq!(cols, width(label, caps), "{label:?} on={on}");
            assert_eq!(cols, crate::chatwidth::str_w(label) + width("", caps));
        }
    }
    assert_eq!(width("", Caps::BOTH), 4, "the chrome alone");
}

/// The page's own colour, floored on the block: dark ink on a bright badge
/// on a dark page, light ink on a light one — and readable on every preset
/// for every tag colour the roster can hand out.
#[test]
fn page_ink_reads_on_every_agent_colour_on_every_preset() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let floor = crew_theme::contrast::text_floor();
        for name in ["planner", "coder", "analyst", "reviewer", "smith"] {
            let bg = crate::chatroster::agent_color(name);
            let ink = page_ink(bg);
            assert!(
                contrast_ratio(ink, bg) >= floor,
                "{id:?} {name}: {ink:?} on {bg:?} = {:.2}",
                contrast_ratio(ink, bg)
            );
        }
    }
}

/// The card adapter keeps colour, weight and block, and stamps no source
/// byte on any cell.
#[test]
fn to_card_carries_the_block_and_no_source() {
    let _g = crate::app::theme_test_guard();
    let b = badge("x", (0, 0, 0), (200, 200, 200), Caps::BOTH);
    let cards = to_card(&b);
    assert_eq!(cards.len(), b.len());
    for (s, c) in b.iter().zip(&cards) {
        assert_eq!((c.c, c.fg, c.bg, c.bold), (s.c, s.fg, s.bg, s.bold));
        assert!(c.src.is_none() && c.link.is_none());
    }
}

/// On a tube every tag colour is a badge dark ink reads on: the bottom tag
/// rung now clears the text floor against the page, so a badge's ink no
/// longer flips to a light grey on the darker tags (`scout` beside `smith`).
/// And a disk tile too dark for that ink takes the tube's own colour, not a
/// neutral grey.
#[test]
fn a_tube_badge_reads_in_dark_ink_and_a_tile_in_its_phosphor() {
    let _g = crate::app::theme_test_guard();
    let lum = |c| crew_theme::contrast_ratio(c, (0, 0, 0));
    for id in crew_theme::ALL_THEMES.into_iter().filter(|id| id.is_crt()) {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        for slot in 0..12 {
            let bg = crew_theme::slot_color(slot, t);
            let ink = super::page_ink(bg);
            assert!(
                lum(ink) < lum(bg),
                "{} slot {slot}: light ink on {bg:?}",
                id.as_str()
            );
        }
        let dark_tile = crate::anim::lerp_rgb(t.page_bg, t.border_focused, 0.3);
        let ink = crate::disktile::label_ink(dark_tile);
        let chroma = crew_theme::oklch::from_srgb(ink).c;
        assert!(
            chroma >= 0.03,
            "{}: tile label {ink:?} is grey",
            id.as_str()
        );
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// On the white-text glass a badge is white words on its colour deepened
/// until white reads — the same hue, never the dark smoke walked onto a
/// pastel block. Off it, the pair is `page_ink` on the colour as given.
#[test]
fn a_clear_glass_badge_is_white_on_its_own_hue_deepened() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let (t, floor) = (crew_theme::theme(), crew_theme::contrast::text_floor());
        // Every tag colour, and the accents — paper-dark's is near-white.
        let accents = [t.accent_default, (240, 240, 240), (255, 255, 255)];
        let tags = (0..12).map(|slot| crew_theme::slot_color(slot, t));
        for (slot, bg) in tags.chain(accents).enumerate() {
            let (ink, fill) = super::inked(bg);
            if !id.is_clear_glass() {
                assert_eq!((ink, fill), (page_ink(bg), bg), "{}", id.as_str());
                continue;
            }
            assert_eq!(ink, t.ink, "{} slot {slot}", id.as_str());
            assert!(
                contrast_ratio(ink, fill) >= floor,
                "{} {slot}: {fill:?}",
                id.as_str()
            );
            let hue = |c| crew_theme::oklch::from_srgb(c).h;
            let dh = (hue(fill) - hue(bg)).abs() % 360.0;
            let grey = crew_theme::oklch::from_srgb(bg).c < 0.03;
            assert!(
                grey || dh.min(360.0 - dh) < 12.0,
                "{} {slot}: hue moved",
                id.as_str()
            );
        }
    }
}
