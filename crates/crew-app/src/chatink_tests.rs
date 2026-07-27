//! The guard that was missing when the semantic palette shipped.
//!
//! `crew-theme`'s `contrast_thresholds` asserts every slot against the PAGE,
//! which the CRT presets passed comfortably while being invisible against
//! body text — `ansi[6]` vs `ink` measured 1.04:1 on crt-green, i.e. the same
//! colour. Contrast against the page says "you can read it"; only contrast
//! against `ink` says "you can tell it apart from prose", and that is the
//! claim the palette actually makes.
use super::*;

/// Every derived colour must be distinguishable from body text in every
/// preset. This is the assertion whose absence let the CRT regression ship.
#[test]
fn semantic_colours_separate_from_body_text() {
    for id in ALL_THEMES {
        let t = id.theme();
        let d = derive(t);
        for (what, c) in [("code", d.code), ("marker", d.marker), ("quote", d.quote)] {
            let got = contrast_ratio(c, t.ink);
            assert!(
                got >= SEPARATION_FLOOR,
                "{}: {what} vs ink = {got:.3} (need >= {SEPARATION_FLOOR})",
                id.as_str(),
            );
        }
    }
}

/// Separation is never bought by fading into the page.
#[test]
fn semantic_colours_stay_readable_on_the_page() {
    for id in ALL_THEMES {
        let t = id.theme();
        let d = derive(t);
        for (what, c) in [("code", d.code), ("marker", d.marker), ("quote", d.quote)] {
            let got = contrast_ratio(c, t.page_bg);
            assert!(
                got >= PAGE_FLOOR,
                "{}: {what} vs page_bg = {got:.3} (need >= {PAGE_FLOOR})",
                id.as_str(),
            );
        }
    }
}

/// The code card has to be visible as a card — including under CRT scanlines
/// and bloom, where the old 0.08 mix measured 1.10:1 and disappeared.
#[test]
fn code_card_reads_as_a_card() {
    for id in ALL_THEMES {
        let t = id.theme();
        let got = contrast_ratio(derive(t).code_bg, t.page_bg);
        assert!(
            got >= 1.3,
            "{}: code_bg vs page_bg = {got:.3} (need >= 1.3)",
            id.as_str(),
        );
    }
}

/// A preset that already separates keeps the colour its author tuned —
/// the floor must not restyle themes that were never broken.
#[test]
fn already_separated_themes_are_untouched() {
    let t = crew_theme::ThemeId::PaperDark.theme();
    if contrast_ratio(t.ansi[6], t.ink) >= SEPARATION_FLOOR {
        assert_eq!(
            separated(t.ansi[6], t),
            t.ansi[6],
            "paper-dark code colour was altered despite already clearing the floor",
        );
    }
}

/// The CRT case that prompted all of this: crt-green's raw `ansi[6]` is the
/// same colour as its `ink`, and must not survive derivation unchanged.
#[test]
fn crt_green_code_colour_moves() {
    let t = crew_theme::ThemeId::CrtGreen.theme();
    let raw = contrast_ratio(t.ansi[6], t.ink);
    assert!(
        raw < 1.2,
        "premise changed: raw crt-green code vs ink = {raw:.3}"
    );
    assert_ne!(
        separated(t.ansi[6], t),
        t.ansi[6],
        "crt-green code colour left at 1.04:1 against body text",
    );
}

/// The derived table is per-preset, not per-call: the active theme must pick
/// its own row rather than always answering with the first one.
#[test]
fn active_theme_selects_its_own_row() {
    let green = derive(crew_theme::ThemeId::CrtGreen.theme());
    let amber = derive(crew_theme::ThemeId::CrtAmber.theme());
    assert_ne!(
        green.code, amber.code,
        "two presets derived the same code colour — the table is not keyed by theme",
    );
}
