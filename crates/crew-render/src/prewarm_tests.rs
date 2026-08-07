//! Prewarm coverage: the working set is the real chrome set, the shaped
//! buffer seeds the swash cache through the smoothing path, the vendored
//! glyphon patches the whole scheme depends on are still applied, and — on
//! a machine with a GPU — `CellGrid::prepare` warms before any scene exists.
use glyphon::cosmic_text::{CacheKey, SwashCache, SwashContent};
use glyphon::FontSystem;

use super::{build_buffer, working_set};
use crate::cellgrid::CellGrid;
use crate::celltext::{cell_metrics, FontParams};
use crate::smoothing::presmooth;

fn params(font_size: f32, smooth: u8) -> FontParams {
    let (cell_w, cell_h) = cell_metrics(font_size);
    FontParams {
        font_size,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 500,
        smooth,
    }
}

/// Every glyph cache key the prewarm buffer shapes, with the cell char that
/// produced it — the exact keys a real frame would look up.
fn shaped_keys(buf: &glyphon::Buffer) -> Vec<(char, CacheKey)> {
    let mut keys = Vec::new();
    for run in buf.layout_runs() {
        for g in run.glyphs.iter() {
            let c = run.text[g.start..g.end].chars().next().unwrap_or(' ');
            keys.push((c, g.physical((0.0, 0.0), 1.0).cache_key));
        }
    }
    keys
}

#[test]
fn working_set_covers_ascii_and_the_chrome_glyphs() {
    let set = working_set();
    for c in ' '..='~' {
        assert!(set.contains(&(c, false)), "missing base-weight {c:?}");
        assert!(set.contains(&(c, true)), "missing bold {c:?}");
    }
    // A sample of every chrome family: rounded pane corners, connectors,
    // bars, spinner frames, markers, prompt/punctuation glyphs.
    for c in "╭╮╯╰─│├└█▍░⠋⠏●⏺✓❯↵—…·".chars() {
        assert!(set.contains(&(c, false)), "missing chrome glyph {c:?}");
    }
    assert!(
        set.len() >= 260,
        "working set shrank to {} entries",
        set.len()
    );
}

#[test]
fn prewarm_buffer_seeds_smoothed_masks_for_every_glyph() {
    let mut fs = FontSystem::new();
    let mut swash = SwashCache::new();
    let p = params(28.0, 200);
    let buffers = [build_buffer(&mut fs, &p)];
    // The same seeding pass `prepare_renderer` (and thus `prewarm`) runs.
    presmooth(&mut swash, &mut fs, &buffers);

    let keys = shaped_keys(&buffers[0].0);
    assert!(keys.len() >= 260, "only {} glyphs shaped", keys.len());
    for (c, key) in &keys {
        assert!(
            swash.image_cache.contains_key(key),
            "prewarm glyph {c:?} was not seeded"
        );
    }
    // The seeds went through smooth_mask, not a raw raster: a sample stroke
    // glyph is 2px padded (1px dilation border per side) vs the raw image.
    let (_, key) = keys
        .iter()
        .find(|(c, _)| *c == 'M')
        .expect("working set has M");
    let seeded = swash
        .image_cache
        .get(key)
        .expect("seeded")
        .clone()
        .expect("M rasterizes");
    let raw = swash.get_image_uncached(&mut fs, *key).expect("M rasters");
    assert_eq!(seeded.content, SwashContent::Mask);
    assert_eq!(seeded.placement.width, raw.placement.width + 2);
    assert_eq!(seeded.placement.height, raw.placement.height + 2);
}

/// The prewarm scheme only helps if the vendored glyphon still (a) starts
/// the mask atlas Retina-sized and (b) uploads through the seeded
/// `SwashCache.image_cache` at BOTH materialization sites. An upgrade that
/// dropped any patch would compile fine and silently regress — this pins
/// the patched source itself.
#[test]
fn vendored_glyphon_patches_are_applied() {
    let atlas_src = include_str!("../../../vendor/glyphon/src/text_atlas.rs");
    let render_src = include_str!("../../../vendor/glyphon/src/text_render.rs");
    assert!(
        atlas_src.contains("Kind::Mask => 1024") && atlas_src.contains("Kind::Color { .. } => 256"),
        "per-kind initial atlas sizes are gone from text_atlas.rs"
    );
    for (name, src) in [("text_atlas.rs", atlas_src), ("text_render.rs", render_src)] {
        assert!(
            src.contains("CREW PATCH"),
            "{name} lost its CREW PATCH marker"
        );
        // A call site (with the paren) — the patch comments themselves may
        // name `get_image_uncached` when explaining what they replaced.
        assert!(
            src.contains(".get_image(") && !src.contains("get_image_uncached("),
            "{name} no longer reads through the seeded image_cache"
        );
    }
}

/// End-to-end on a real device: a fresh `CellGrid` warms the swash cache
/// (and thus the atlas — `prepare_renderer` uploads whatever it seeds) on
/// its first `prepare`, before any scene is set, through the smoothed path;
/// font changes re-arm it. Skips on GPU-less CI like the headless tests.
#[test]
fn cellgrid_prepare_prewarms_before_any_scene() {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("cellgrid_prepare_prewarms_before_any_scene: no GPU adapter, skipping");
            return;
        }
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    // 28px ≈ the default 14pt at Retina 2× — the size the atlas math targets.
    let mut grid = CellGrid::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm, 28.0);
    assert!(grid.swash.image_cache.is_empty(), "cache dirty before warm");
    let t0 = std::time::Instant::now();
    grid.prepare(&device, &queue, 640, 480);
    eprintln!("prewarm: first prepare (no scene) took {:?}", t0.elapsed());

    // Re-shape the working set with the grid's own params: every key a real
    // frame would look up must already be seeded. RED if the prewarm call in
    // `prepare` is removed (nothing else populates the cache — no scene) or
    // if it rasterized through some other path (different keys/None seeds).
    let p = grid.font_params();
    let t1 = std::time::Instant::now();
    let (buf, ..) = build_buffer(&mut grid.font_system, &p);
    eprintln!("prewarm: shaping alone (re-shape) took {:?}", t1.elapsed());
    let keys = shaped_keys(&buf);
    assert!(keys.len() >= 260, "only {} glyphs shaped", keys.len());
    for (c, key) in &keys {
        assert!(
            grid.swash.image_cache.contains_key(key),
            "prewarm missed {c:?}"
        );
    }
    // Smoothed, not raw: the seeded 'M' carries the 2px dilation padding.
    let (_, key) = keys.iter().find(|(c, _)| *c == 'M').expect("has M");
    let seeded = grid
        .swash
        .image_cache
        .get(key)
        .expect("seeded")
        .clone()
        .expect("M rasterizes");
    let raw = SwashCache::new()
        .get_image_uncached(&mut grid.font_system, *key)
        .expect("M rasters raw");
    assert_eq!(seeded.placement.width, raw.placement.width + 2);
    assert_eq!(seeded.placement.height, raw.placement.height + 2);

    // The warm is one-shot until a font-affecting knob re-arms it.
    assert!(!grid.needs_prewarm, "prepare must consume the arm flag");
    grid.set_font_size(30.0);
    assert!(grid.needs_prewarm, "font changes must re-arm the prewarm");
}
