use glyphon::cosmic_text::{CacheKeyFlags, SwashCache, SwashContent, SwashImage};

use super::{presmooth, smooth_mask, strength_of, text_flags};
use crate::cellgrid::CellView;
use crate::celltext::{build_pane_buffer, cell_metrics, FontParams};

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
fn text_flags_disable_hinting_and_carry_the_strength_byte() {
    let flags = text_flags(137);
    assert!(flags.contains(CacheKeyFlags::DISABLE_HINTING));
    assert_eq!((flags.bits() >> 8) & 0xFF, 137);
    // Strength 0 still disables hinting — the CoreText look is unhinted
    // even with the stem darkening turned off.
    assert!(text_flags(0).contains(CacheKeyFlags::DISABLE_HINTING));
    assert_eq!(text_flags(0).bits() >> 8, 0);
}

#[test]
fn shaped_glyphs_carry_the_flags_through_to_their_cache_keys() {
    let mut fs = crate::embedfont::font_system();
    let (cell_w, cell_h) = cell_metrics(14.0);
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
    // Horizontal neighbours get full strength (255), vertical half (127),
    // diagonals nothing — the dilation is a 4-neighbourhood.
    #[rustfmt::skip]
    let expected = vec![
          0, 127,   0,
        255, 255, 255,
          0, 127,   0,
    ];
    assert_eq!(out.data, expected);
}

#[test]
fn dilation_scales_linearly_with_strength() {
    let out = smooth_mask(&mask_1x1(255), 100);
    #[rustfmt::skip]
    let expected = vec![
          0,  50,   0,
        100, 255, 100,
          0,  50,   0,
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

#[test]
fn presmooth_seeds_the_cache_with_padded_heavier_masks() {
    let mut fs = crate::embedfont::font_system();
    let mut swash = SwashCache::new();
    let (cell_w, cell_h) = cell_metrics(14.0);
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
    let (cell_w, cell_h) = cell_metrics(14.0);
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
