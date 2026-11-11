use glyphon::cosmic_text::{CacheKey, SwashCache, SwashImage};
use glyphon::FontSystem;

use super::render;
use crate::cellgrid::CellView;
use crate::celltext::{build_pane_buffer, cell_metrics, FontParams, CELL_H_RATIO};
use crate::smoothing::{grow_px, DEFAULT_SMOOTH};

const LETTERS: [char; 8] = ['l', 'o', 'e', 'H', 'n', 'a', 's', 'k'];

/// The cache key crew shapes `c` to at `px`, hinting off as every run has it.
fn key_of(fs: &mut FontSystem, c: char, px: f32) -> CacheKey {
    let (cell_w, cell_h) = cell_metrics(px, CELL_H_RATIO);
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
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
    };
    let buf = build_pane_buffer(fs, &cells, 1, 1, cell_w, cell_h, &p);
    let glyph = buf.layout_runs().flat_map(|r| r.glyphs.to_vec()).next();
    glyph
        .expect("one glyph")
        .physical((0.0, 0.0), 1.0)
        .cache_key
}

fn ink(img: &SwashImage) -> f64 {
    img.data.iter().map(|&a| f64::from(a)).sum::<f64>() / 255.0
}

/// Ink-weighted centre, in the pixel space the placement puts the mask in.
fn centre(img: &SwashImage) -> (f64, f64) {
    let w = img.placement.width as usize;
    let (mut sx, mut sy) = (0f64, 0f64);
    for (i, &a) in img.data.iter().enumerate() {
        sx += f64::from(a) * ((i % w) as f64 + f64::from(img.placement.left));
        sy += f64::from(a) * ((i / w) as f64 - f64::from(img.placement.top));
    }
    let total = ink(img) * 255.0;
    (sx / total, sy / total)
}

/// The share of a mask's inked pixels sitting on the faint half of the edge.
fn faint_share(imgs: &[SwashImage]) -> f64 {
    let inked = |lo: u8, hi: u8| {
        let hit = |i: &&SwashImage| i.data.iter().filter(|&&a| a > lo && a <= hi).count();
        imgs.iter().map(|i| hit(&i)).sum::<usize>() as f64
    };
    inked(10, 102) / inked(10, 255)
}

/// With nothing to grow this IS cosmic-text's rasterizer: the copy of its
/// scaler set-up here (size, hinting, the weight axis) has not drifted.
#[test]
fn an_ungrown_glyph_is_the_one_cosmic_text_draws() {
    let mut fs = crate::embedfont::font_system();
    let mut swash = SwashCache::new();
    for px in [13.0, 24.0] {
        for c in LETTERS {
            let key = key_of(&mut fs, c, px);
            let ours = render(&mut fs, key, 0.0).expect("rasterizes");
            let theirs = swash.get_image_uncached(&mut fs, key).expect("rasterizes");
            assert_eq!(ours.data, theirs.data, "{c:?} at {px} px");
            assert_eq!(
                (ours.placement.left, ours.placement.top),
                (theirs.placement.left, theirs.placement.top)
            );
        }
    }
}

/// Growing the outline adds ink on both sides of a stem alike, so a glyph
/// gets heavier where it stands instead of drifting to the right — and on
/// top of a bar only, so its foot stays on the baseline (CoreText's shape).
#[test]
fn a_grown_glyph_is_heavier_and_stands_where_it_stood() {
    let mut fs = crate::embedfont::font_system();
    let grow = grow_px(DEFAULT_SMOOTH, 24.0, true);
    for c in LETTERS {
        let key = key_of(&mut fs, c, 24.0);
        let raw = render(&mut fs, key, 0.0).expect("rasterizes");
        let grown = render(&mut fs, key, grow).expect("rasterizes");
        assert!(ink(&grown) > ink(&raw) * 1.05, "{c:?} gained no weight");
        let (a, b) = (centre(&raw), centre(&grown));
        let (dx, up) = ((a.0 - b.0).abs(), a.1 - b.1);
        // Left uncentred a glyph drifts half the grow; a lopsided letter's
        // own shape (the arms of `k`) accounts for well under half of that.
        let drift = f64::from(grow) / 4.0;
        assert!(dx < drift, "{c:?} moved {dx:.3} px sideways when it grew");
        assert!(
            (0.0..f64::from(grow)).contains(&up),
            "{c:?} rose {up:.3} px"
        );
        let foot = |i: &SwashImage| i.placement.height as i32 - i.placement.top;
        assert_eq!(foot(&grown), foot(&raw), "{c:?} left the baseline");
    }
}

/// What the widening is on the outline FOR (reported 2026-11-11, "fonts are
/// not smooth enough"): the weight must not be paid for with the edge. The
/// bitmap kernel this replaced left 2% of inked pixels on the faint half of
/// the ramp where the raw outline has 20%; a grown outline keeps its share.
#[test]
fn a_grown_glyph_keeps_its_soft_edge() {
    let mut fs = crate::embedfont::font_system();
    let mut at = |grow: f32| -> Vec<SwashImage> {
        let one = |c| {
            let key = key_of(&mut fs, c, 24.0);
            render(&mut fs, key, grow).expect("rasterizes")
        };
        LETTERS.into_iter().map(one).collect()
    };
    let (raw, grown) = (at(0.0), at(grow_px(DEFAULT_SMOOTH, 24.0, true)));
    let (before, after) = (faint_share(&raw), faint_share(&grown));
    assert!(
        before > 0.12,
        "the raw outline has no ramp to keep: {before:.3}"
    );
    assert!(
        after > before * 0.7,
        "the edge went hard: {after:.3} of inked pixels are faint, from {before:.3}"
    );
}

/// The grow follows the glyph's size, as CoreText's does: body text on a
/// 1x display and on a Retina one take the same share of extra weight, so
/// small text is not drowned and large text is not left thin.
#[test]
fn small_and_large_text_gain_the_same_share_of_weight() {
    let mut fs = crate::embedfont::font_system();
    let mut gain = |px: f32| {
        let (mut raw, mut grown) = (0.0, 0.0);
        for c in LETTERS {
            let key = key_of(&mut fs, c, px);
            raw += ink(&render(&mut fs, key, 0.0).expect("rasterizes"));
            let grow = grow_px(DEFAULT_SMOOTH, px, true);
            grown += ink(&render(&mut fs, key, grow).expect("rasterizes"));
        }
        (grown - raw) * 100.0 / raw
    };
    let (small, body, retina) = (gain(10.0), gain(14.0), gain(24.0));
    println!("gain: {small:.1}% at 10 px, {body:.1}% at 14, {retina:.1}% at 24");
    for (px, g) in [(10, small), (14, body)] {
        assert!(
            (g - retina).abs() <= 5.0,
            "{px} px gained {g:.1}%, 24 px {retina:.1}%"
        );
    }
}

/// CoreText widens dark ink on a bright page about half as far as light ink
/// on a dark one, stops following the size where CoreText does, and `off`
/// widens nothing either way up.
#[test]
fn the_grow_follows_polarity_size_and_strength() {
    let dark = grow_px(DEFAULT_SMOOTH, 24.0, true);
    assert!(
        (0.5..0.8).contains(&dark),
        "medium widens a stroke {dark} px"
    );
    assert_eq!(grow_px(DEFAULT_SMOOTH, 24.0, false), dark * 0.5);
    assert_eq!(grow_px(DEFAULT_SMOOTH, 12.0, true), dark * 0.5);
    assert_eq!(
        grow_px(DEFAULT_SMOOTH, 96.0, true),
        dark,
        "light ink caps at 24 px"
    );
    assert_eq!(
        grow_px(DEFAULT_SMOOTH, 96.0, false),
        dark,
        "dark ink caps at 48 px"
    );
    assert_eq!(grow_px(0, 24.0, true), 0.0);
    assert_eq!(grow_px(0, 24.0, false), 0.0);
}
