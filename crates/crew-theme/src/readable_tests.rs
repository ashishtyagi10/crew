use super::*;
use crate::ALL_THEMES;

/// The whole point: every one of these roles clears its floor on EVERY theme,
/// light and dark alike. Before this module, six of them failed on all three
/// light pages — three of them badly enough to be invisible.
#[test]
fn every_role_clears_its_floor_on_every_theme() {
    let mut bad: Vec<String> = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        let block = cursor(t, true);
        for (name, fg, bg, floor) in [
            ("cursor (focused)", block, t.term_bg, TEXT_FLOOR),
            ("glyph on the cursor", on_block(t, block), block, TEXT_FLOOR),
            ("link", link(t), t.term_bg, TEXT_FLOOR),
            ("selection", selection_bg(t), t.term_fg, TEXT_FLOOR),
            ("warn", warn(t), t.page_bg, TEXT_FLOOR),
            ("danger", danger(t), t.page_bg, TEXT_FLOOR),
            ("spark", spark(t), t.page_bg, MARK_FLOOR),
        ] {
            let r = contrast_ratio(fg, bg);
            if r < floor {
                bad.push(format!("{}: {name} is {r:.2}, floor {floor}", id.as_str()));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
}

/// The defect that made the cursor worth fixing first: on a light page the
/// FOCUSED cursor read at 1.5 and the unfocused ones at 6.2, so the pane you
/// were typing in had the faintest cursor on the canvas. The focused cursor
/// must out-read the unfocused one on every theme, by a margin you can see.
#[test]
fn the_focused_cursor_always_out_reads_the_unfocused_one() {
    for id in ALL_THEMES {
        let t = id.theme();
        let bg = t.term_bg;
        let (on, off) = (
            contrast_ratio(cursor(t, true), bg),
            contrast_ratio(cursor(t, false), bg),
        );
        assert!(
            on > off * 1.5,
            "{}: focused {on:.2} vs unfocused {off:.2} — the signal is not clear",
            id.as_str()
        );
    }
}

/// An unfocused cursor must still be *there*. Fixing the inversion by making
/// the unfocused one vanish would trade one defect for another.
#[test]
fn the_unfocused_cursor_is_still_visible() {
    for id in ALL_THEMES {
        let t = id.theme();
        let r = contrast_ratio(cursor(t, false), t.term_bg);
        assert!(r >= 1.6, "{}: unfocused cursor at {r:.2}", id.as_str());
    }
}

/// Hue is meaning and survives: a link stays blue, a warning amber, an alarm
/// red. Only lightness is the palette's to take.
#[test]
fn a_role_keeps_its_hue_when_the_page_takes_its_lightness() {
    for id in ALL_THEMES {
        let t = id.theme();
        for (name, got, want) in [
            ("link", link(t), (90u8, 170u8, 255u8)),
            ("warn", warn(t), (230, 180, 90)),
            ("danger", danger(t), (230, 90, 90)),
        ] {
            let (a, b) = (oklch::from_srgb(got).h, oklch::from_srgb(want).h);
            let d = (a - b).abs().min(360.0 - (a - b).abs());
            assert!(d < 8.0, "{}: {name} hue moved {d:.1}°", id.as_str());
        }
    }
}

/// A colour that already clears its floor is returned untouched — the dark
/// pages these were chosen on must look exactly as they did.
#[test]
fn a_colour_that_already_reads_is_left_alone() {
    let page = (12, 8, 5);
    let want = (90, 170, 255);
    assert!(contrast_ratio(want, page) >= TEXT_FLOOR, "premise");
    assert_eq!(against(want, page, TEXT_FLOOR), want);
}

/// The walk goes the right way on each kind of page, and stops as soon as it
/// clears rather than running to the pole.
#[test]
fn the_walk_moves_away_from_the_page_and_stops_when_it_clears() {
    let want = (90, 170, 255);
    let light = (246, 243, 236);
    let got = against(want, light, TEXT_FLOOR);
    assert!(
        oklch::from_srgb(got).l < oklch::from_srgb(want).l,
        "a light page must darken it"
    );
    assert!(contrast_ratio(got, light) >= TEXT_FLOOR);
    // …and not all the way to black: it stops at the first step that clears.
    assert!(oklch::from_srgb(got).l > 0.15, "it overshot to {got:?}");

    let dark = (12, 8, 5);
    let dim = (30, 30, 40);
    let got = against(dim, dark, TEXT_FLOOR);
    assert!(
        oklch::from_srgb(got).l > oklch::from_srgb(dim).l,
        "a dark page must lighten it"
    );
}

/// A hue with nowhere left to go returns the best it reached rather than
/// looping or returning the failing input.
#[test]
fn an_impossible_floor_returns_the_best_reachable_colour() {
    let page = (128, 128, 128);
    let got = against((130, 130, 130), page, 21.0);
    assert!(
        contrast_ratio(got, page) > contrast_ratio((130, 130, 130), page),
        "it should still have improved on {got:?}"
    );
}
