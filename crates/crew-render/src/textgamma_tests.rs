use super::{build, Curve, DEFAULT_TEXT_GAMMA, GAMMA};

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
