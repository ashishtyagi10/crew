use super::{amount_for, build, contrast_factor, Curve, DEFAULT_TEXT_GAMMA, GAMMA};
/// Both curves must fix the endpoints: an empty pixel stays empty and a
/// solid one stays solid, at every amount and either polarity. Only the
/// antialiased rim is the blend's problem, and only the rim may move.
#[test]
fn endpoints_never_move() {
    for dark in [true, false] {
        for amount in [1u8, 60, DEFAULT_TEXT_GAMMA, 255] {
            let lut = build(dark, amount);
            assert_eq!(lut[0], 0, "dark={dark} amount={amount}");
            assert_eq!(lut[255], 255, "dark={dark} amount={amount}");
        }
    }
}

/// A curve that reorders coverage would shred the antialiasing. Every step
/// must be non-decreasing.
#[test]
fn the_curve_is_monotone() {
    for dark in [true, false] {
        for amount in [1u8, DEFAULT_TEXT_GAMMA, 255] {
            let lut = build(dark, amount);
            for i in 1..256 {
                assert!(
                    lut[i] >= lut[i - 1],
                    "dark={dark} amount={amount}: {} -> {} at {i}",
                    lut[i - 1],
                    lut[i]
                );
            }
        }
    }
}

/// The polarities bend opposite ways, and that is the whole point: light ink
/// on a dark page has lost luminance to the encoded blend and must get it
/// back; dark ink on a bright page has gained it and must give it up.
#[test]
fn polarity_decides_which_way_the_rim_bends() {
    let dark = build(true, DEFAULT_TEXT_GAMMA);
    let light = build(false, DEFAULT_TEXT_GAMMA);
    assert!(
        dark[128] > 128,
        "dark page must lift the rim: {}",
        dark[128]
    );
    assert!(
        light[128] < 128,
        "bright page must lower the rim: {}",
        light[128]
    );
    // Symmetric about the midpoint: the two corrections are mirror images.
    assert!(
        (i16::from(dark[128]) - 128).abs_diff(128 - i16::from(light[128])) <= 1,
        "corrections must mirror: {} vs {}",
        dark[128],
        light[128]
    );
}

/// At full amount the curve IS the sRGB transfer function — that is what
/// "the full physical correction" means, and it is what makes a dark page
/// deliver the linear luminance the coverage asked for.
#[test]
fn full_amount_restores_the_linear_luminance() {
    let lut = build(true, 255);
    for i in [32usize, 64, 128, 200] {
        let corrected = f32::from(lut[i]) / 255.0;
        // The blend will raise this to GAMMA; that must land back on `a`.
        let delivered = corrected.powf(GAMMA);
        let want = i as f32 / 255.0;
        assert!(
            (delivered - want).abs() < 0.005,
            "coverage {i}: delivers {delivered:.4}, wanted {want:.4}"
        );
    }
}

/// Amount 0 is the identity, and must not even touch the bytes — `/gamma
/// off` has to mean the mask that came out of the rasterizer.
#[test]
fn amount_zero_is_the_identity() {
    let mut data = vec![0u8, 17, 64, 128, 200, 255];
    let before = data.clone();
    Curve::new().apply(&mut data, true, 0);
    assert_eq!(data, before);
}

/// The curve is remembered across glyphs but must re-derive the moment the
/// pair changes — a theme switch flips polarity mid-session.
#[test]
fn the_curve_follows_a_polarity_switch() {
    let mut curve = Curve::new();
    let mut on_dark = vec![128u8];
    curve.apply(&mut on_dark, true, DEFAULT_TEXT_GAMMA);
    let mut on_light = vec![128u8];
    curve.apply(&mut on_light, false, DEFAULT_TEXT_GAMMA);
    assert!(
        on_dark[0] > 128 && on_light[0] < 128,
        "cached curve leaked across the switch: {} then {}",
        on_dark[0],
        on_light[0]
    );
}

/// Polarity is a property of a cell's own two colours, not of the theme.
/// Crew draws bright badges inside dark themes, dark chips inside light
/// ones, and inverts both under the cursor — and a curve bent the wrong way
/// does not merely fail to help, it doubles the error it was there to
/// cancel.
#[test]
fn polarity_reads_the_cells_own_colours() {
    use super::light_ink;
    let dark_page = (18, 18, 20);
    let bright_badge = (240, 238, 230);
    let pale_ink = (230, 230, 235);
    let ink = (20, 20, 24);
    assert!(light_ink(pale_ink, dark_page), "light ink on the dark page");
    assert!(
        !light_ink(ink, bright_badge),
        "dark ink on a bright badge, inside the same dark theme"
    );
    // Inverting a cell — what the cursor does — inverts the answer.
    assert!(light_ink(dark_page, pale_ink) != light_ink(pale_ink, dark_page));
}

/// Luminance, not byte value: a saturated green reads brighter than a
/// saturated blue of the same numeric level, and the curve has to agree
/// with the eye rather than with the tuple.
#[test]
fn polarity_weighs_the_channels_the_way_the_eye_does() {
    let green = (0, 200, 0);
    let blue = (0, 0, 200);
    assert!(
        super::light_ink(green, blue),
        "green ink on blue ground is light ink"
    );
    assert!(!super::light_ink(blue, green));
}

/// The curve `a^(1/2.2)` is exact for the extreme pair and only that pair.
/// Anything narrower loses proportionally less light to the encoded blend,
/// so it asks for proportionally less correction.
#[test]
fn only_the_extreme_pair_asks_for_the_whole_correction() {
    let white = (255, 255, 255);
    let black = (0, 0, 0);
    assert!(
        (contrast_factor(white, black) - 1.0).abs() < 0.01,
        "white on black is the pair the curve was derived for"
    );
    assert!(
        (contrast_factor(black, white) - 1.0).abs() < 0.01,
        "and so is its mirror"
    );
    // Crew's own dark body pair sits well inside it.
    let body = contrast_factor((228, 224, 216), (24, 20, 17));
    assert!(
        (0.7..0.95).contains(&body),
        "crew's body pair asks for {body:.3} of the correction"
    );
    // A muted comment on the same page is narrower still, and must ask for
    // less than the body text beside it — the whole point of measuring per
    // run rather than per theme.
    let muted = contrast_factor((120, 114, 104), (24, 20, 17));
    assert!(
        muted < body,
        "muted {muted:.3} must ask for less than body {body:.3}"
    );
    // Ink the colour of its own ground has no rim to correct at all.
    assert_eq!(contrast_factor((80, 80, 80), (80, 80, 80)), 0.0);
}

/// The factor scales the AMOUNT, which is already a byte in every glyph's
/// cache key — that is what lets the correction vary per run with nothing
/// plumbed anywhere.
#[test]
fn the_amount_scales_with_the_pair_and_stays_a_byte() {
    assert_eq!(amount_for(255, (255, 255, 255), (0, 0, 0)), 255);
    assert_eq!(amount_for(0, (255, 255, 255), (0, 0, 0)), 0);
    let body = amount_for(255, (228, 224, 216), (24, 20, 17));
    assert!((178..=242).contains(&body), "body pair amount {body}");
    assert_eq!(amount_for(255, (80, 80, 80), (80, 80, 80)), 0);
}

/// What the scaling is FOR: the full correction lifts a stroke's outermost
/// pixel far more than crew's actual pages ask for, and that lift is a halo.
#[test]
fn the_scaled_curve_lifts_a_faint_rim_less_than_the_full_one() {
    let mut full = Curve::new();
    let mut scaled = Curve::new();
    let (fg, bg) = ((228, 224, 216), (24, 20, 17));
    let rim = |c: &mut Curve, amount: u8| {
        let mut d = [13u8];
        c.apply(&mut d, true, amount);
        d[0]
    };
    let f = rim(&mut full, 255);
    let s = rim(&mut scaled, amount_for(255, fg, bg));
    assert!(
        f > s,
        "the full correction lifts 13 to {f}, the pair's to {s}"
    );
    assert!(s > 13, "but the rim is still corrected, not dropped");
}
