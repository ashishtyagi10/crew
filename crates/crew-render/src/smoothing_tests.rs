use glyphon::cosmic_text::{
    fontdb, CacheKey, CacheKeyFlags, SubpixelBin, SwashCache, SwashContent,
};

use super::{dark_of, gamma_of, presmooth, strength_of, text_flags};
use crate::cellgrid::CellView;
use crate::celltext::CELL_H_RATIO;
use crate::celltext::{build_pane_buffer, cell_metrics, FontParams};

#[test]
fn text_flags_disable_hinting_and_carry_the_strength_byte() {
    let flags = text_flags(137, 0, false);
    assert!(flags.contains(CacheKeyFlags::DISABLE_HINTING));
    assert_eq!((flags.bits() >> 8) & 0xFF, 137);
    // Strength 0 still disables hinting — the CoreText look is unhinted
    // even with the stem darkening turned off.
    assert!(text_flags(0, 0, false).contains(CacheKeyFlags::DISABLE_HINTING));
    assert_eq!((text_flags(0, 0, false).bits() >> 8) & 0xFF, 0);
}

/// The three smoothing parameters share one 32-bit flag word, so they have
/// to occupy disjoint fields: a strength must never read back as a gamma
/// amount, and neither may set the polarity bit.
#[test]
fn the_flag_word_keeps_its_three_fields_apart() {
    for &(s, c, d) in &[
        (0u8, 0u8, false),
        (255, 0, true),
        (0, 255, false),
        (137, 42, true),
    ] {
        let flags = text_flags(s, c, d);
        let key = CacheKey {
            font_id: fontdb::ID::dummy(),
            glyph_id: 0,
            font_size_bits: 0,
            font_weight: fontdb::Weight::NORMAL,
            x_bin: SubpixelBin::Zero,
            y_bin: SubpixelBin::Zero,
            flags,
        };
        assert_eq!(strength_of(&key), s, "strength for {s}/{c}/{d}");
        assert_eq!(gamma_of(&key), c, "gamma for {s}/{c}/{d}");
        assert_eq!(dark_of(&key), d, "polarity for {s}/{c}/{d}");
        assert!(flags.contains(CacheKeyFlags::DISABLE_HINTING));
    }
}

#[test]
fn shaped_glyphs_carry_the_flags_through_to_their_cache_keys() {
    let mut fs = crate::embedfont::font_system();
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let cells = [CellView {
        col: 0,
        row: 0,
        c: 'M',
        fg: (255, 255, 255),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    }];
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 400,
        smooth: 137,
        gamma: 0,
        dark: true,
    };
    let buf = build_pane_buffer(&mut fs, &cells, 1, 1, cell_w, cell_h, &p);
    let glyph = buf
        .layout_runs()
        .flat_map(|r| r.glyphs.to_vec())
        .next()
        .expect("one glyph");
    let key = glyph.physical((0.0, 0.0), 1.0).cache_key;
    assert!(key.flags.contains(CacheKeyFlags::DISABLE_HINTING));
    assert_eq!(strength_of(&key), 137);
}

#[test]
fn presmooth_seeds_the_cache_with_padded_heavier_masks() {
    let mut fs = crate::embedfont::font_system();
    let mut swash = SwashCache::new();
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let cells = [CellView {
        col: 0,
        row: 0,
        c: 'M',
        fg: (255, 255, 255),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    }];
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 400,
        smooth: 200,
        gamma: 0,
        dark: true,
    };
    let buf = build_pane_buffer(&mut fs, &cells, 1, 1, cell_w, cell_h, &p);
    let buffers = vec![(buf, 0.0f32, 0.0f32, cell_w, cell_h)];
    presmooth(&mut swash, &mut fs, &buffers);

    // Compare the seeded 'M' against a raw re-raster of the same key.
    let key = buffers[0]
        .0
        .layout_runs()
        .flat_map(|r| r.glyphs.to_vec())
        .next()
        .expect("one glyph")
        .physical((0.0, 0.0), 1.0)
        .cache_key;
    let seeded = swash
        .image_cache
        .get(&key)
        .expect("presmooth seeded the key")
        .clone()
        .expect("M rasterizes");
    let raw = swash
        .get_image_uncached(&mut fs, key)
        .expect("M rasterizes raw");
    assert_eq!(seeded.content, SwashContent::Mask);
    assert_eq!(seeded.placement.width, raw.placement.width + 2);
    assert_eq!(seeded.placement.height, raw.placement.height + 2);
    let coverage = |d: &[u8]| d.iter().map(|&a| u64::from(a)).sum::<u64>();
    assert!(
        coverage(&seeded.data) > coverage(&raw.data),
        "stem darkening must add coverage: {} vs {}",
        coverage(&seeded.data),
        coverage(&raw.data)
    );
}

#[test]
fn presmooth_at_zero_strength_seeds_untouched_masks() {
    let mut fs = crate::embedfont::font_system();
    let mut swash = SwashCache::new();
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let cells = [CellView {
        col: 0,
        row: 0,
        c: 'M',
        fg: (255, 255, 255),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    }];
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 400,
        smooth: 0,
        gamma: 0,
        dark: true,
    };
    let buf = build_pane_buffer(&mut fs, &cells, 1, 1, cell_w, cell_h, &p);
    let buffers = vec![(buf, 0.0f32, 0.0f32, cell_w, cell_h)];
    presmooth(&mut swash, &mut fs, &buffers);
    let key = buffers[0]
        .0
        .layout_runs()
        .flat_map(|r| r.glyphs.to_vec())
        .next()
        .expect("one glyph")
        .physical((0.0, 0.0), 1.0)
        .cache_key;
    let seeded = swash
        .image_cache
        .get(&key)
        .expect("presmooth seeded the key")
        .clone()
        .expect("M rasterizes");
    let raw = swash
        .get_image_uncached(&mut fs, key)
        .expect("M rasterizes raw");
    assert_eq!(
        (seeded.placement.left, seeded.placement.top),
        (raw.placement.left, raw.placement.top)
    );
    assert_eq!(
        (seeded.placement.width, seeded.placement.height),
        (raw.placement.width, raw.placement.height)
    );
    assert_eq!(seeded.data, raw.data);
}
