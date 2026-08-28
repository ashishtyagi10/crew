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
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
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

/// Diff line classes take the theme's RAW slots, matching the viewer's diff
/// rung (`viewpane::lines::diff_lines`) — a ```diff fence in chat and an
/// opened .patch file must colour identically.
#[test]
fn diff_tokens_match_the_viewers_diff_rung() {
    use crate::md::syntax::Token;
    // Two reads of the process-global theme (here and inside `token_fg`):
    // serialised against the theme-mutating tests or a mid-test `/theme`
    // switch makes the two disagree under the parallel runner.
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    assert_eq!(token_fg(Token::Added), t.ansi[2], "added is green");
    assert_eq!(token_fg(Token::Removed), t.ansi[1], "removed is red");
    assert_eq!(token_fg(Token::Hunk), t.ansi[6], "hunk header is cyan");
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

/// Syntax colours are subject to the same floor as everything else: a
/// highlighted keyword that matches body text is the v0.6.34 bug again, with
/// four colours instead of one.
#[test]
fn every_syntax_colour_separates_from_body_text() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let t = id.theme();
        let d = derive(t);
        for (what, c) in [
            ("comment", d.comment),
            ("string", d.string),
            ("keyword", d.keyword),
        ] {
            let got = contrast_ratio(c, t.ink);
            assert!(
                got >= SEPARATION_FLOOR,
                "{}: {what} vs ink = {got:.3} (need >= {SEPARATION_FLOOR})",
                id.as_str(),
            );
            // Comments alone answer to the lower floor — see
            // `COMMENT_PAGE_FLOOR`. They are the class meant to recede.
            let want = if what == "comment" { 3.5 } else { PAGE_FLOOR };
            let page = contrast_ratio(c, t.page_bg);
            assert!(
                page >= want,
                "{}: {what} vs page_bg = {page:.3} (need >= {want})",
                id.as_str(),
            );
        }
    }
}

/// What the ladder actually guarantees, stated in the terms the metric can
/// express.
///
/// `contrast_ratio` is a LUMINANCE ratio: it cannot see hue at all. Green
/// string text on teal code text measures ~1.04 on the light presets and looks
/// nothing alike — so a high ratio is the wrong thing to demand where the
/// theme can vary hue, and the right thing to demand where it cannot.
///
/// The single-phosphor CRT presets are exactly that case: one hue, so
/// lightness is the only axis highlighting has, and the ladder must earn its
/// keep there in the only currency available.
#[test]
fn the_syntax_ladder_holds_where_hue_cannot_help() {
    let _g = crate::app::theme_test_guard();
    let mut tubes = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        let d = derive(t);
        let comment = contrast_ratio(d.comment, d.code);
        assert!(
            comment >= 1.15,
            "{}: comment vs code = {comment:.3}",
            id.as_str(),
        );
        // "Single-phosphor" means the CRT palettes, not everything carrying a
        // `CrtStyle`: the modern family carries a bloom-only one purely as a
        // halo vehicle (`RandomMode::Crt` draws the same line with the same
        // guard) and has a full 16-slot palette, so hue does the separating
        // there and the stiffer rung would only distort its cyan.
        //
        // Spelled `is_tube` — the theme's own predicate — because every
        // preset now carries a `ModernStyle`, and the `modern.is_some()`
        // spelling this used to have excluded all twelve. The stiffer rungs
        // below had not been asserted on anything for as long as that was
        // true; the counter at the end is what says so out loud.
        if !t.is_tube() {
            continue;
        }
        tubes += 1;
        assert!(
            comment >= 1.6,
            "{} is single-phosphor, so lightness is all there is: comment vs \
             code = {comment:.3}",
            id.as_str(),
        );
        let string = contrast_ratio(d.string, d.code);
        assert!(
            string >= 1.2,
            "{} is single-phosphor: string vs code = {string:.3}",
            id.as_str(),
        );
    }
    assert_eq!(tubes, 4, "every tube was actually checked");
}

/// A fenced block is a rectangle of `code_bg` — that field IS the block, now
/// that no corner glyphs draw one. So it has to be visible on every preset,
/// and it has to stay a backdrop: on the tubes a fixed 0.18 mix measured
/// 1.39:1 against the page, which bloom and scanlines finish off.
#[test]
fn the_code_field_reads_on_every_preset_without_swallowing_its_code() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        let d = derive(t);
        let field = contrast_ratio(d.code_bg, t.page_bg);
        assert!(
            field >= FIELD_FLOOR,
            "{}: code field vs page = {field:.3}",
            id.as_str(),
        );
        let on_field = contrast_ratio(d.code, d.code_bg);
        assert!(
            on_field >= CODE_ON_FIELD_FLOOR,
            "{}: code vs its own field = {on_field:.3}",
            id.as_str(),
        );
    }
}
