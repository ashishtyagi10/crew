//! Colour-space maths is the kind of code that looks right and is wrong by a
//! transposed matrix row, so these check against values that came from
//! *outside* this file — Ottosson's published reference conversions and the
//! CSS Color 4 spec examples — rather than against the code's own output.
use super::*;

fn close(a: f32, b: f32, eps: f32, what: &str) {
    assert!(
        (a - b).abs() < eps,
        "{what}: got {a:.5}, expected {b:.5} (Δ {:.5} > {eps})",
        (a - b).abs()
    );
}

/// Published OKLab values for the sRGB primaries (Ottosson's article, and the
/// same numbers CSS Color 4 lists). If a matrix row were transposed these
/// would be wrong while every round-trip in this file still passed.
#[test]
fn the_srgb_primaries_match_published_oklch_values() {
    // (rgb, expected L, expected C, expected H°)
    let cases = [
        ((255, 255, 255), 1.000, 0.000, None),
        ((0, 0, 0), 0.000, 0.000, None),
        ((255, 0, 0), 0.628, 0.258, Some(29.2)),
        ((0, 255, 0), 0.866, 0.295, Some(142.5)),
        ((0, 0, 255), 0.452, 0.313, Some(264.1)),
    ];
    for (rgb, l, c, h) in cases {
        let got = from_srgb(rgb);
        close(got.l, l, 0.003, &format!("{rgb:?} lightness"));
        close(got.c, c, 0.003, &format!("{rgb:?} chroma"));
        if let Some(h) = h {
            close(got.h, h, 0.5, &format!("{rgb:?} hue"));
        }
    }
}

/// Grey must be exactly neutral: any chroma here would tint every derived
/// neutral role in every theme.
#[test]
fn greys_have_no_chroma() {
    for v in [0u8, 32, 64, 128, 192, 255] {
        let got = from_srgb((v, v, v));
        assert!(
            got.c < 1e-3,
            "grey {v} has chroma {:.5} — neutrals would come out tinted",
            got.c
        );
    }
}

/// Perceptual uniformity, stated as the property the ramp depends on: equal
/// OKLCH lightness reads as equal lightness *across hues*. HSL fails this
/// badly — which is the entire reason for this module.
#[test]
fn equal_lightness_across_hues_lands_within_a_narrow_luminance_band() {
    let l = 0.65;
    let lums: Vec<f32> = (0..12)
        .map(|i| {
            let rgb = Oklch::new(l, 0.10, i as f32 * 30.0).to_srgb();
            crate::contrast_ratio(rgb, (0, 0, 0))
        })
        .collect();
    let (lo, hi) = lums
        .iter()
        .fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
    // Same L, twelve hues: contrast against black stays within ~12%.
    assert!(
        hi / lo < 1.12,
        "equal-lightness hues span {lo:.2}..{hi:.2} contrast ({:.0}%) — not \
         perceptually uniform enough to derive a palette from",
        (hi / lo - 1.0) * 100.0
    );
}

#[test]
fn srgb_round_trips_through_oklch() {
    // A spread including the actual page colours of both pools.
    for rgb in [
        (15, 17, 23),    // AURORA page
        (250, 247, 240), // a paper page
        (200, 30, 90),
        (0, 128, 255),
        (7, 7, 7),
        (248, 248, 248),
    ] {
        let back = from_srgb(rgb).to_srgb();
        let d = distance(rgb, back);
        assert!(
            d < 0.004,
            "{rgb:?} round-tripped to {back:?} (Δ {d:.5}) — the conversion is lossy"
        );
    }
}

/// Out-of-gamut colours must lose *chroma*, not hue. Clipping channels — the
/// obvious implementation — swings hue visibly, which would show up as a
/// palette whose accent drifts colour as it gets brighter.
#[test]
fn an_out_of_gamut_colour_loses_chroma_not_hue() {
    // Far beyond what sRGB can show at this lightness.
    let want = Oklch::new(0.55, 0.40, 264.0);
    let got = from_srgb(want.to_srgb());
    close(got.h, want.h, 2.0, "hue after gamut reduction");
    close(got.l, want.l, 0.02, "lightness after gamut reduction");
    assert!(
        got.c < want.c,
        "chroma was not reduced ({:.3} vs {:.3}) — the colour cannot be in gamut",
        got.c,
        want.c
    );
}

#[test]
fn toward_reads_the_page() {
    assert_eq!(Toward::for_page((15, 17, 23)), Toward::Light);
    assert_eq!(Toward::for_page((250, 247, 240)), Toward::Dark);
}

/// The inversion the ramp rests on: ask for a ratio, get a colour that has it.
#[test]
fn the_solver_hits_every_floor_the_suite_asserts() {
    // The real floors from `contrast_thresholds`, on both pools' pages.
    let pages = [(15, 17, 23), (250, 247, 240), (7, 7, 7), (255, 255, 255)];
    let floors = [10.0, 7.0, 4.5, 3.0, 2.5, 2.3, 2.2, 1.45];
    for page in pages {
        let toward = Toward::for_page(page);
        for target in floors {
            for hue in [0.0, 90.0, 180.0, 264.0, 330.0] {
                let got = solve_for_contrast(page, hue, 0.06, target, toward);
                let ratio = crate::contrast_ratio(got, page);
                assert!(
                    ratio >= target - 0.02,
                    "page {page:?} hue {hue} target {target}: solved {got:?} = \
                     {ratio:.3}, short of the floor"
                );
            }
        }
    }
}

/// …and lands *at* the floor rather than sailing past it. Without this the
/// hierarchy between ink, muted and hint collapses toward maximum contrast and
/// every theme reads the same.
#[test]
fn the_solver_stops_at_the_floor_instead_of_maximising() {
    let page = (15, 17, 23);
    let toward = Toward::for_page(page);
    let ink = solve_for_contrast(page, 264.0, 0.02, 10.0, toward);
    let muted = solve_for_contrast(page, 264.0, 0.02, 7.0, toward);
    let hint = solve_for_contrast(page, 264.0, 0.02, 2.5, toward);

    let (ri, rm, rh) = (
        crate::contrast_ratio(ink, page),
        crate::contrast_ratio(muted, page),
        crate::contrast_ratio(hint, page),
    );
    assert!(
        ri > rm && rm > rh,
        "roles must stay ordered: {ri} > {rm} > {rh}"
    );
    assert!(
        rm < 8.0 && rh < 3.2,
        "solved past the floor (muted {rm:.2}, hint {rh:.2}) — the hierarchy \
         flattens if every role maximises contrast"
    );
}

/// A floor nothing can reach must still return the most legible colour there
/// is, not a panic and not something dimmer.
#[test]
fn an_impossible_floor_returns_the_extreme() {
    // Nothing on a white page reaches 30:1.
    let got = solve_for_contrast((255, 255, 255), 0.0, 0.0, 30.0, Toward::Dark);
    assert_eq!(
        got,
        (0, 0, 0),
        "should fall back to black, the best available"
    );
}

/// The distance scale, anchored to colours crew already ships rather than to
/// invented thresholds. Every number below was measured from the current
/// presets, so this test doubles as the documentation for what a Δ means here
/// — which is what Phase 2's pairwise ANSI check will be calibrated against.
#[test]
fn the_distance_scale_matches_crew_s_own_palette_steps() {
    let d = distance;
    assert!(
        d((10, 20, 30), (10, 20, 30)) < 1e-6,
        "a colour is zero from itself"
    );

    // Below one 8-bit code: invisible.
    assert!(d((128, 128, 128), (129, 128, 128)) < 0.003);
    // Eight codes of grey: visible as a step, not as a different colour.
    let step = d((128, 128, 128), (136, 136, 136));
    assert!(
        (0.02..0.04).contains(&step),
        "8-code grey step measured {step:.4}"
    );

    // One rung of crew's own text hierarchy — AURORA ink → text_muted. This is
    // the anchor: ~0.10 is "clearly a different role, same family".
    let rung = d((232, 235, 243), (196, 203, 217));
    assert!(
        (0.08..0.12).contains(&rung),
        "AURORA ink→muted measured {rung:.4}; the 0.10 anchor these tests and \
         the ramp are calibrated on has moved"
    );

    // Two ANSI hues a user must never confuse — AURORA red vs yellow.
    let hues = d((242, 139, 130), (253, 214, 99));
    assert!(hues > 0.15, "ansi red vs yellow measured {hues:.4}");

    // The extremes bound the scale at 1.0.
    assert!((d((0, 0, 0), (255, 255, 255)) - 1.0).abs() < 0.01);

    // …and it is ordered throughout.
    assert!(d((128, 128, 128), (129, 128, 128)) < step && step < rung && rung < hues);
}
