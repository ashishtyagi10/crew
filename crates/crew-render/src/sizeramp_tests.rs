use super::{size_scale, strength_at, CALIBRATED_PX, MIN_SIZE_SCALE};

#[test]
fn the_size_ramp_is_one_sided_monotone_and_floored() {
    // At and above the reference the user's number is used untouched — the
    // ladder means what it says at body size and every size above it.
    for px in [CALIBRATED_PX, 14.5, 18.0, 28.0, 96.0] {
        assert_eq!(size_scale(px), 1.0, "{px} px must not be scaled");
        assert_eq!(strength_at(170, px), 170);
    }
    // Below it, monotone down to the floor and never past it.
    let mut prev = 1.0;
    for px in [13.0f32, 12.0, 11.0, 10.0, 9.0, 7.0, 5.0, 2.0, 0.5] {
        let s = size_scale(px);
        assert!(
            s <= prev,
            "{px} px scaled {s}, above the larger size's {prev}"
        );
        assert!(s >= MIN_SIZE_SCALE, "{px} px fell through the floor: {s}");
        prev = s;
    }
    // Zero strength stays zero at every size; the ramp only ever scales.
    assert_eq!(strength_at(0, 6.0), 0);
}

/// What the ramp is for: the darkening must not run ahead of its
/// calibration as the text shrinks. Before it, a run of body letters gained
/// 39% ink at 9 px against 31% at the 14 px the ladder was tuned at, and
/// the surplus came out of the counters.
#[test]
fn small_text_takes_the_same_share_of_darkening_as_body_text() {
    use crate::cellgrid::CellView;
    use crate::celltext::{build_pane_buffer, cell_metrics, FontParams, CELL_H_RATIO};
    use glyphon::cosmic_text::SwashCache;

    let mut fs = crate::embedfont::font_system();
    let mut swash = SwashCache::new();
    let gain = |px: f32, fs: &mut _, swash: &mut SwashCache| {
        let (cell_w, cell_h) = cell_metrics(px, CELL_H_RATIO);
        let (mut raw_ink, mut smoothed_ink) = (0f64, 0f64);
        for c in ['l', 'o', 'e', 'H', 'n', 'a'] {
            let cells = [CellView {
                col: 0,
                row: 0,
                c,
                fg: (255, 255, 255),
                bg: (0, 0, 0),
                ..Default::default()
            }];
            let p = FontParams {
                font_size: px,
                line_height: cell_h,
                cell_w,
                family: None,
                weight: 500,
                smooth: crate::smoothing::DEFAULT_SMOOTH,
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
            let strength = strength_at(
                crate::smoothing::DEFAULT_SMOOTH,
                f32::from_bits(key.font_size_bits),
            );
            let smoothed = crate::smoothmask::smooth_mask(&raw, strength);
            let ink = |d: &[u8]| d.iter().map(|&a| u64::from(a)).sum::<u64>() as f64;
            raw_ink += ink(&raw.data);
            smoothed_ink += ink(&smoothed.data);
        }
        (smoothed_ink - raw_ink) * 100.0 / raw_ink
    };
    let small = gain(9.0, &mut fs, &mut swash);
    let body = gain(14.0, &mut fs, &mut swash);
    assert!(
        (small - body).abs() <= 3.0,
        "9 px gained {small:.1}% ink against body text's {body:.1}% — the \
         darkening is running ahead of its calibration at the small end"
    );
}
