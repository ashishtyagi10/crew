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
