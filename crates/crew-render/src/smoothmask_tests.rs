use glyphon::cosmic_text::{SwashContent, SwashImage};

use super::smooth_mask;

fn mask_1x1(alpha: u8) -> SwashImage {
    let mut image = SwashImage::new();
    image.content = SwashContent::Mask;
    image.placement.left = 3;
    image.placement.top = 5;
    image.placement.width = 1;
    image.placement.height = 1;
    image.data = vec![alpha];
    image
}

#[test]
fn full_strength_dilation_of_a_lone_pixel() {
    let out = smooth_mask(&mask_1x1(255), 255);
    // 1×1 grows to 3×3 with the placement shifted to compensate: one column
    // left (left 3→2) and one row up (top 5→6, top counts upward).
    assert_eq!(
        (
            out.placement.left,
            out.placement.top,
            out.placement.width,
            out.placement.height
        ),
        (2, 6, 3, 3)
    );
    // Max strength spills 0.70 px horizontally (SPILL_SCALE) and half that
    // vertically; diagonals get nothing — the spill runs over a
    // 4-neighbourhood.
    #[rustfmt::skip]
    let expected = vec![
          0,  90,   0,
        179, 255, 179,
          0,  90,   0,
    ];
    assert_eq!(out.data, expected);
}

#[test]
fn dilation_scales_linearly_with_strength() {
    let out = smooth_mask(&mask_1x1(255), 100);
    #[rustfmt::skip]
    let expected = vec![
          0,  35,   0,
         70, 255,  70,
          0,  35,   0,
    ];
    assert_eq!(out.data, expected);
}

#[test]
fn smoothing_never_dims_an_original_pixel() {
    let mut image = mask_1x1(0);
    image.placement.width = 3;
    image.placement.height = 2;
    image.data = vec![10, 200, 30, 0, 255, 90];
    let out = smooth_mask(&image, 180);
    assert_eq!((out.placement.width, out.placement.height), (5, 4));
    for y in 0..2usize {
        for x in 0..3usize {
            let original = image.data[y * 3 + x];
            let smoothed = out.data[(y + 1) * 5 + (x + 1)];
            assert!(
                smoothed >= original,
                "pixel ({x},{y}) dimmed: {original} -> {smoothed}"
            );
        }
    }
}

/// A pixel that already carries partial coverage sits on every curve and
/// diagonal flank. A saturating `max()` dilation leaves those pixels alone
/// whenever their own coverage already beats the neighbour's spill, so `o`
/// and `/` gain almost none of the widening `l` and `H` get.
#[test]
fn partially_covered_flanks_gain_coverage() {
    let mut image = mask_1x1(0);
    image.placement.width = 3;
    image.placement.height = 1;
    image.data = vec![0, 200, 255];
    let out = smooth_mask(&image, 100);
    // The padded bitmap is 5×3; the original row sits at y = 1, x = 1..4.
    let got = out.data[5 + 2];
    assert!(got > 200, "flank pixel must darken: 200 -> {got}");
}

/// The accumulation must stay bounded: full coverage plus any spill is
/// still full coverage, never a wrapped byte.
#[test]
fn accumulated_coverage_saturates_at_full() {
    let mut image = mask_1x1(0);
    image.placement.width = 3;
    image.placement.height = 1;
    image.data = vec![255, 254, 255];
    let out = smooth_mask(&image, 255);
    assert_eq!(&out.data[5 + 1..5 + 4], &[255, 255, 255]);
}

/// The point of accumulating rather than saturating: a glyph built from
/// curves must take nearly as much of the darkening as one built from
/// stems. With the old `max()` kernel `s` gained only 82% of what `l` did,
/// which is why round letters read a shade lighter than upright ones in
/// the same word. Measured on the embedded font at the default body size.
///
/// Pinned to a fixed strength rather than the default: this is a property of
/// the KERNEL, and the default is now 0 (see
/// `the_default_pair_delivers_the_outlines_light`) — the ladder still runs
/// through this code for anyone who turns it on.
/// A representative `/smooth` setting to exercise the kernel at.
const KERNEL_STRENGTH: u8 = 70;

#[test]
fn curves_take_nearly_as_much_darkening_as_stems() {
    use crate::cellgrid::CellView;
    use crate::celltext::{build_pane_buffer, cell_metrics, FontParams, CELL_H_RATIO};
    use glyphon::cosmic_text::SwashCache;

    let mut fs = crate::embedfont::font_system();
    let mut swash = SwashCache::new();
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let ink = |d: &[u8]| d.iter().map(|&a| u64::from(a)).sum::<u64>() as f64;
    let gain = |c: char, fs: &mut _, swash: &mut SwashCache| {
        let cells = [CellView {
            col: 0,
            row: 0,
            c,
            fg: (255, 255, 255),
            bg: (0, 0, 0),
            ..Default::default()
        }];
        let p = FontParams {
            font_size: 14.0,
            line_height: cell_h,
            cell_w,
            family: None,
            weight: 400,
            smooth: KERNEL_STRENGTH,
            gamma: 0,
            dark: true,
            body: ((255, 255, 255), (0, 0, 0)),
        };
        let buf = build_pane_buffer(fs, &cells, 1, 1, cell_w, cell_h, &p);
        let key = buf
            .layout_runs()
            .flat_map(|r| r.glyphs.to_vec())
            .next()
            .expect("one glyph")
            .physical((0.0, 0.0), 1.0)
            .cache_key;
        let raw = SwashCache::get_image_uncached(swash, fs, key).expect("rasterizes");
        let smoothed = smooth_mask(&raw, KERNEL_STRENGTH);
        (ink(&smoothed.data) - ink(&raw.data)) / ink(&raw.data)
    };
    let curved = gain('s', &mut fs, &mut swash);
    let stem = gain('l', &mut fs, &mut swash);
    assert!(
        curved / stem >= 0.90,
        "curved glyphs left behind: s gained {curved:.3}, l gained {stem:.3} \
         (ratio {:.3}, want >= 0.90)",
        curved / stem
    );
}

/// The contract the two text defaults are set against: together they land on
/// the outline's own light, on BOTH polarities, and they put that light on as
/// few pixels as it can go on.
///
/// Two corrections have stood here. The stem darkening came first, when it
/// was the only one, and its calibration was quietly covering the encoded
/// blend's deficit as well as doing its own optical widening. `textgamma`
/// took that job over honestly, 0.19.28 rebalanced the pair so they stopped
/// stacking, and this measurement is what asks the question that rebalance
/// did not: whether the darkening still earns anything.
///
/// It does not. `smooth 0, gamma 255` delivers 100% of the asked light both
/// ways up on 322 inked pixels; `smooth 70, gamma 130` delivered 98% on a
/// dark page, **145% on a bright one**, and needed 584 pixels to do it. The
/// 262 extra were fractions of a stem's coverage sitting a pixel out from
/// the stem, which is a soft edge with nothing bought for it.
///
/// The bright-page number is the one that had never been looked at: this
/// test only ever rendered white ink on a black page.
#[test]
fn the_default_pair_delivers_the_outlines_light() {
    use crate::cellgrid::CellView;
    use crate::celltext::{build_pane_buffer, cell_metrics, FontParams, CELL_H_RATIO};
    use glyphon::cosmic_text::SwashCache;

    let mut fs = crate::embedfont::font_system();
    let mut swash = SwashCache::new();
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let asked = |d: &[u8]| d.iter().map(|&a| f64::from(a) / 255.0).sum::<f64>();
    // What the encoded blend actually emits for the coverage crew hands it:
    // light ink raises the stored alpha to the display gamma, dark ink on a
    // bright page has the same error with the sign flipped.
    let delivered = |d: &[u8], dark: bool| {
        d.iter()
            .map(|&a| {
                let a = f64::from(a) / 255.0;
                if dark {
                    a.powf(2.2)
                } else {
                    1.0 - (1.0 - a).powf(2.2)
                }
            })
            .sum::<f64>()
    };
    for dark in [true, false] {
        let (mut want, mut got, mut inked, mut outline_px) = (0.0, 0.0, 0usize, 0usize);
        for c in ['l', 'o', 'e', 'H', 'n', 'a', 's', 't'] {
            let (fg, bg) = if dark {
                ((255, 255, 255), (0, 0, 0))
            } else {
                ((0, 0, 0), (255, 255, 255))
            };
            let cells = [CellView {
                col: 0,
                row: 0,
                c,
                fg,
                bg,
                ..Default::default()
            }];
            let p = FontParams {
                font_size: 14.0,
                line_height: cell_h,
                cell_w,
                family: None,
                weight: 500,
                smooth: crate::smoothing::DEFAULT_SMOOTH,
                gamma: crate::textgamma::DEFAULT_TEXT_GAMMA,
                dark,
                body: ((255, 255, 255), (0, 0, 0)),
            };
            let buf = build_pane_buffer(&mut fs, &cells, 1, 1, cell_w, cell_h, &p);
            let key = buf
                .layout_runs()
                .flat_map(|r| r.glyphs.to_vec())
                .next()
                .expect("one glyph")
                .physical((0.0, 0.0), 1.0)
                .cache_key;
            let raw = swash.get_image_uncached(&mut fs, key).expect("rasterizes");
            want += asked(&raw.data);
            outline_px += raw.data.iter().filter(|a| **a > 0).count();
            let strength = crate::sizeramp::strength_at(crate::smoothing::DEFAULT_SMOOTH, 14.0);
            let mut img = if strength > 0 {
                smooth_mask(&raw, strength)
            } else {
                raw.clone()
            };
            crate::textgamma::Curve::new().apply(
                &mut img.data,
                dark,
                crate::textgamma::DEFAULT_TEXT_GAMMA,
            );
            got += delivered(&img.data, dark);
            inked += img.data.iter().filter(|a| **a > 0).count();
        }
        let pct = got * 100.0 / want;
        assert!(
            (97.0..=103.0).contains(&pct),
            "on a {} page the defaults deliver {pct:.1}% of the outline's light",
            if dark { "dark" } else { "bright" }
        );
        // Nothing may put ink on a pixel the outline did not reach. A
        // dilation does exactly that, and every pixel it adds is a soft edge.
        assert!(
            inked <= outline_px,
            "the defaults ink {inked} pixels where the outline reached {outline_px}"
        );
    }
}
