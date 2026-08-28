use super::*;
use crate::ALL_THEMES;

/// The whole point: every one of these roles clears its floor on EVERY theme,
/// light and dark alike. Before this module, six of them failed on all three
/// light pages — three of them badly enough to be invisible.
#[test]
fn every_role_clears_its_floor_on_every_theme() {
    let _g = crate::contrast::test_lock();
    crate::contrast::set_high_contrast(false);
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
    let _g = crate::contrast::test_lock();
    crate::contrast::set_high_contrast(false);
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
    let _g = crate::contrast::test_lock();
    crate::contrast::set_high_contrast(false);
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
    let _g = crate::contrast::test_lock();
    crate::contrast::set_high_contrast(false);
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

/// The whole contract again, one band up: with the OS asking for contrast,
/// every role has to clear the RAISED floor on every theme. If a role could
/// not reach AAA on some page the feature would be a promise crew cannot keep
/// — and the failure would be silent, since `against` returns its best effort
/// rather than erroring.
#[test]
fn every_role_clears_the_raised_floor_too() {
    let _g = crate::contrast::test_lock();
    crate::contrast::set_high_contrast(true);
    let mut bad: Vec<String> = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        let block = cursor(t, true);
        for (name, fg, bg, floor) in [
            ("cursor (focused)", block, t.term_bg, 7.0),
            ("glyph on the cursor", on_block(t, block), block, 7.0),
            ("link", link(t), t.term_bg, 7.0),
            ("selection", selection_bg(t), t.term_fg, 7.0),
            ("warn", warn(t), t.page_bg, 7.0),
            ("danger", danger(t), t.page_bg, 7.0),
            ("spark", spark(t), t.page_bg, 4.5),
        ] {
            let r = contrast_ratio(fg, bg);
            if r < floor {
                bad.push(format!("{}: {name} is {r:.2}, floor {floor}", id.as_str()));
            }
        }
    }
    crate::contrast::set_high_contrast(false);
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
}

/// The switch has to actually reach the colours. A floor that rose while every
/// derived role came back byte-identical would be the feature shipped as a
/// no-op — and on a dark page, where most of these already clear AA
/// comfortably, that is exactly how it would look.
#[test]
fn asking_for_contrast_actually_moves_the_derived_colours() {
    let _g = crate::contrast::test_lock();
    let mut moved = 0;
    let mut checked = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        // Each role against the background it is actually measured on —
        // `link` lands on the terminal page, `selection_bg` under the
        // terminal's own ink, the rest on the app page.
        crate::contrast::set_high_contrast(false);
        let aa = [
            (link(t), t.term_bg),
            (selection_bg(t), t.term_fg),
            (warn(t), t.page_bg),
            (danger(t), t.page_bg),
            (spark(t), t.page_bg),
        ];
        crate::contrast::set_high_contrast(true);
        let aaa = [
            (link(t), t.term_bg),
            (selection_bg(t), t.term_fg),
            (warn(t), t.page_bg),
            (danger(t), t.page_bg),
            (spark(t), t.page_bg),
        ];
        for (&(a, bg), &(b, _)) in aa.iter().zip(aaa.iter()) {
            checked += 1;
            if a != b {
                moved += 1;
            }
            // Never the wrong way: the raised floor may leave a colour alone
            // (it already cleared AAA) but must never make it read worse.
            assert!(
                contrast_ratio(b, bg) >= contrast_ratio(a, bg) - 0.01,
                "{}: a raised floor lowered a ratio",
                id.as_str()
            );
        }
    }
    crate::contrast::set_high_contrast(false);
    assert!(
        moved * 2 > checked,
        "only {moved} of {checked} roles moved — the floor is not reaching them"
    );
}

/// `secondary` is quieter than what it is given but never invisible: it keeps
/// the hue (so it still reads as the same measurement) and never falls under
/// the mark floor (so it is still a mark).
#[test]
fn secondary_is_quieter_but_never_below_the_mark_floor() {
    let _g = crate::contrast::test_lock();
    crate::contrast::set_high_contrast(false);
    let mut bad: Vec<String> = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        for (name, want) in [
            ("accent", t.accent_default),
            ("warn", warn(t)),
            ("danger", danger(t)),
        ] {
            let q = secondary(want, t.page_bg);
            let (full, quiet) = (
                contrast_ratio(want, t.page_bg),
                contrast_ratio(q, t.page_bg),
            );
            if quiet >= full {
                bad.push(format!(
                    "{}: {name} not quieter ({quiet:.2} vs {full:.2})",
                    id.as_str()
                ));
            }
            if quiet < MARK_FLOOR - 0.01 {
                bad.push(format!(
                    "{}: {name} is {quiet:.2}, floor {MARK_FLOOR}",
                    id.as_str()
                ));
            }
            // Same measurement, so the same hue — a chromatic colour must not
            // walk into a different one on its way down.
            let (a, b) = (crate::oklch::from_srgb(want), crate::oklch::from_srgb(q));
            let dh = (a.h - b.h).abs().min(360.0 - (a.h - b.h).abs());
            if a.c > 0.02 && dh > 4.0 {
                bad.push(format!("{}: {name} hue moved {dh:.1} degrees", id.as_str()));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
}
