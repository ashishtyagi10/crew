//! CoreText-style glyph smoothing for the swash raster path.
//!
//! Ghostty, Warp, kitty and Terminal.app all rasterize through CoreText,
//! which never hints and (with "font smoothing" on) dilates strokes by a
//! fraction of a pixel — Apple's stem darkening. swash hints by default and
//! draws exact outlines, so the same font reads thinner and rougher in crew
//! than in any native terminal. This module closes that gap:
//!
//! - every text run carries [`CacheKeyFlags::DISABLE_HINTING`] (CoreText
//!   never hints, and unhinted outlines are what subpixel binning expects),
//! - the smoothing strength rides in the cache key's spare high bits, so a
//!   `/smooth` change mints new keys and stale atlas entries simply age out,
//! - [`presmooth`] rasterizes every glyph a frame needs *before* glyphon
//!   does, applies [`smooth_mask`], and pre-fills `SwashCache.image_cache`
//!   (a public field) — glyphon's own `get_image` then hits our entry, so
//!   no fork of glyphon or cosmic-text is needed.
//!
//! The kernel that does the darkening lives in [`crate::smoothmask`].
use glyphon::cosmic_text::{CacheKey, CacheKeyFlags, SwashCache, SwashContent};
use glyphon::FontSystem;

use crate::scene::PaneBuffer;
use crate::smoothmask::smooth_mask;

/// Default smoothing strength (0–255). Calibrated against Terminal.app's
/// default look: ~0.4 px of horizontal stem widening at full coverage.
pub const DEFAULT_SMOOTH: u8 = 100;

/// Strength byte's position inside the cache-key flag bits. Bits 0–2 are
/// cosmic-text's own flags; 8..16 are unused by it and survive the trip
/// through shaping untouched (`from_bits_retain` keeps unknown bits).
const STRENGTH_SHIFT: u32 = 8;

/// Cache-key flags for one text run: hinting always off (the CoreText look
/// is unhinted at every DPI) plus the strength byte for [`presmooth`].
pub(crate) fn text_flags(strength: u8) -> CacheKeyFlags {
    CacheKeyFlags::from_bits_retain(
        CacheKeyFlags::DISABLE_HINTING.bits() | (u32::from(strength) << STRENGTH_SHIFT),
    )
}

/// The strength byte a shaped glyph carries in its cache key.
fn strength_of(key: &CacheKey) -> u8 {
    (key.flags.bits() >> STRENGTH_SHIFT) as u8
}

/// Rasterize and smooth every glyph `buffers` will need, seeding the shared
/// swash cache so glyphon uploads the smoothed bitmaps. Must run before
/// `TextRenderer::prepare` with the same origins the `TextArea`s use; keys
/// already in the cache were seeded by an earlier frame and are skipped.
pub(crate) fn presmooth(
    swash: &mut SwashCache,
    font_system: &mut FontSystem,
    buffers: &[PaneBuffer],
) {
    for (buf, ox, oy, _, _) in buffers {
        for run in buf.layout_runs() {
            for glyph in run.glyphs.iter() {
                let key = glyph.physical((*ox, *oy), 1.0).cache_key;
                if swash.image_cache.contains_key(&key) {
                    continue;
                }
                let image = swash.get_image_uncached(font_system, key).map(|image| {
                    match (image.content, strength_of(&key)) {
                        // Alpha masks (regular text) get the stem darkening;
                        // color emoji and zero strength pass through as-is.
                        (SwashContent::Mask, s) if s > 0 => smooth_mask(&image, s),
                        _ => image,
                    }
                });
                swash.image_cache.insert(key, image);
            }
        }
    }
}

#[cfg(test)]
#[path = "smoothing_tests.rs"]
mod tests;
