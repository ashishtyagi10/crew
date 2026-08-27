use super::*;

#[test]
fn a_named_gradient_is_a_ramp_from_one_pole_to_the_other() {
    let (a, b) = crew_theme::gradients::by_name("aurora").unwrap();
    let chips = for_value("/gradient", "aurora");
    assert_eq!(chips.len(), RAMP);
    assert_eq!(chips[0].fg, a, "the ramp does not start at the first pole");
    assert_eq!(chips[RAMP - 1].fg, b, "nor end at the second");
    // …and travels between them rather than jumping.
    let mid = chips[1].fg;
    assert!(mid != a && mid != b, "the middle is not interpolated");
    assert!(
        (mid.0 as i32 - a.0 as i32).abs() < (b.0 as i32 - a.0 as i32).abs(),
        "the second cell is past the halfway point"
    );
}

/// The colourless gradient is four identical cells — the swatch tells the
/// truth about a pair that has nowhere to travel.
#[test]
fn the_colourless_gradient_is_flat() {
    let chips = for_value("/gradient", "mono");
    assert_eq!(chips.len(), RAMP);
    assert!(chips.windows(2).all(|w| w[0].fg == w[1].fg));
}

/// A freeform `#rrggbb` pair is not on the shelf; nothing is drawn for it
/// rather than a swatch of some other gradient.
#[test]
fn an_unknown_gradient_name_has_no_swatch() {
    assert!(for_value("/gradient", "#ff0000 #00ff00").is_empty());
    assert!(for_value("/gradient", "off").is_empty());
    assert!(for_value("/gradient", "").is_empty());
}

/// A mode shows the palettes it will actually rotate through — one chip each,
/// and pools with different members look different.
#[test]
fn a_theme_mode_shows_one_chip_per_palette_in_its_pool() {
    let dark = for_value("/theme", "dark");
    let crt = for_value("/theme", "crt");
    assert!(dark.len() > 1, "the dark pool has more than one palette");
    assert_eq!(
        crt.len(),
        ALL_THEMES.iter().filter(|id| id.is_crt()).count()
    );
    let fgs = |v: &[Chip]| v.iter().map(|c| c.fg).collect::<Vec<_>>();
    assert_ne!(fgs(&dark), fgs(&crt), "two pools drew the same swatch");
}

/// Each chip carries two colours — that palette's page underneath and one of
/// its faces on top — because a dark pool's pages are all nearly black and a
/// column of them would be one smudge.
#[test]
fn a_chip_carries_both_the_page_and_a_face() {
    let chips = for_value("/theme", "crt-green");
    let t = ThemeId::CrtGreen.theme();
    assert!(!chips.is_empty());
    for (i, c) in chips.iter().enumerate() {
        assert_eq!(c.bg, Some(t.page_bg), "chip {i} left its page");
        assert_ne!(c.fg, t.page_bg, "chip {i} is a single flat colour");
    }
}

/// A named palette shows its HAND, not one chip: the ink it writes in, its
/// accent, and the four ANSI slots every program in a pane paints with. One
/// chip is enough to pick a pool out of four and far too little to pick a
/// palette out of twelve — the dark pool's pages are all nearly black, and
/// their accents are the one colour a user is most likely to have overridden.
#[test]
fn a_named_palette_shows_the_colours_that_tell_it_apart() {
    let t = ThemeId::CrtGreen.theme();
    let fgs: Vec<(u8, u8, u8)> = for_value("/theme", "crt-green")
        .iter()
        .map(|c| c.fg)
        .collect();
    assert_eq!(
        fgs,
        vec![
            t.ink,
            t.accent_default,
            t.ansi[1],
            t.ansi[2],
            t.ansi[3],
            t.ansi[4]
        ]
    );
}

/// The strip is what separates two palettes, so no two palettes may draw the
/// same one. Every pair, every release — a new palette that lands on another's
/// colours is a row the picker cannot tell you anything with.
#[test]
fn no_two_palettes_draw_the_same_strip() {
    let strip = |id: ThemeId| {
        for_value("/theme", id.as_str())
            .iter()
            .map(|c| (c.fg, c.bg))
            .collect::<Vec<_>>()
    };
    for (i, a) in ALL_THEMES.iter().enumerate() {
        assert!(!strip(*a).is_empty(), "{} draws nothing", a.as_str());
        for b in &ALL_THEMES[i + 1..] {
            assert_ne!(strip(*a), strip(*b), "{} and {}", a.as_str(), b.as_str());
        }
    }
}

/// A pinned palette and the mode that rotates it are both nameable at
/// `/theme`, and both draw something.
#[test]
fn every_theme_value_the_picker_offers_draws_a_swatch() {
    for m in crew_theme::THEME_MODES {
        assert!(
            !for_value("/theme", m.as_str()).is_empty(),
            "{} has no swatch",
            m.as_str()
        );
    }
    for g in crew_theme::gradients::GRADIENTS {
        assert_eq!(for_value("/gradient", g.name).len(), RAMP, "{}", g.name);
    }
}

/// Values that are not colours draw nothing — the column belongs to the ones
/// that are.
#[test]
fn a_value_that_is_not_a_colour_has_no_swatch() {
    assert!(for_value("/crt", "on").is_empty());
    assert!(for_value("/weight", "bold").is_empty());
    assert!(for_value("/motion", "full").is_empty());
}

/// The accent field holds a hex colour, which is its own swatch.
#[test]
fn a_hex_value_is_a_chip_and_anything_else_is_not() {
    assert_eq!(hex_chip("#ff8800").map(|c| c.fg), Some((255, 136, 0)));
    assert_eq!(hex_chip("  #00FF00 ").map(|c| c.fg), Some((0, 255, 0)));
    for no in ["", "#fff", "ff8800", "#gggggg", "#ff88000", "follow"] {
        assert!(hex_chip(no).is_none(), "{no} became a chip");
    }
}

/// Every face in the strip is drawn ON that palette's own page, so every face
/// has to be legible against it — which is the same question the terminal
/// asks, since these are the slots every program in a pane paints with. The
/// floor is 3.0, the one crew holds marks and non-text signals to.
///
/// A chip nobody can see is worse than no chip: it says the palette HAS no
/// such colour.
#[test]
fn every_face_reads_against_its_own_page() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let page = id.theme().page_bg;
        for (i, chip) in for_value("/theme", id.as_str()).iter().enumerate() {
            let cr = crew_theme::contrast_ratio(chip.fg, page);
            assert!(
                cr >= 3.0,
                "{}: face {i} reads at {cr:.2} on its own page",
                id.as_str()
            );
        }
    }
}
