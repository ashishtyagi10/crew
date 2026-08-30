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
//! The kernel that does the darkening lives in [`crate::smoothmask`], and
//! the coverage curve that follows it in [`crate::textgamma`] — the flags
//! carry that curve's amount and the page's polarity too, for the same
//! reason they carry the strength: a theme switch or a `/gamma` change
//! has to re-key every glyph or the atlas would keep serving the old ink.
use glyphon::cosmic_text::{CacheKey, CacheKeyFlags, SwashCache, SwashContent};
use glyphon::FontSystem;

use crate::scene::PaneBuffer;
use crate::sizeramp::strength_at;
use crate::smoothmask::smooth_mask;
use crate::textgamma::Curve;

/// Default smoothing strength (0–255) — **off**.
///
/// The stem darkening was added when it was the only correction crew had,
/// and it was doing two jobs: its own optical widening, and quietly covering
/// the deficit of the gamma-encoded blend. [`crate::textgamma`] took the
/// second job over honestly in 0.19.25, and 0.19.28 rebalanced the pair to
/// stop them double-counting. What that rebalance did not ask is whether the
/// darkening was still earning its place at all.
///
/// Swept over eight glyphs at two sizes and both polarities, it is not:
///
/// | pair | light delivered | inked pixels |
/// |------|-----------------|--------------|
/// | smooth 70, gamma 130 (the old default) | 98% dark / **145% light** | 584 |
/// | smooth 0, gamma 255 | **100% / 100%** | **322** |
///
/// The curve alone lands on the outline's own light exactly, on both a dark
/// page and a bright one, and puts that light on 45% FEWER pixels. Every one
/// of those extra pixels is a fraction of a stem's coverage sitting one
/// pixel out from the stem — which is the definition of a soft edge. (The
/// bright-page overshoot had never been measured: the calibration contract
/// only ever rendered white ink on a black page, and the same pair delivers
/// half again the ink it should the other way up.)
///
/// `/smooth` keeps the whole ladder for anyone who wants the fatter
/// Terminal.app look back; what changed is which end of it is the default.
/// `the_default_pair_delivers_the_outlines_light` holds both polarities now.
pub const DEFAULT_SMOOTH: u8 = 0;

/// Strength byte's position inside the cache-key flag bits. Bits 0–2 are
/// cosmic-text's own flags; 8..16 are unused by it and survive the trip
/// through shaping untouched (`from_bits_retain` keeps unknown bits).
const STRENGTH_SHIFT: u32 = 8;

/// The text-gamma amount's position in the same spare region.
const GAMMA_SHIFT: u32 = 16;

/// Set when the page is dark, i.e. the ink is light — which way the coverage
/// curve bends.
const DARK_BIT: u32 = 1 << 24;

/// Cache-key flags for one text run: hinting always off (the CoreText look
/// is unhinted at every DPI) plus everything [`presmooth`] needs to finish
/// the glyph — the darkening strength, the gamma amount, and the page
/// polarity that decides which way the gamma curve bends.
pub(crate) fn text_flags(strength: u8, gamma: u8, dark: bool) -> CacheKeyFlags {
    CacheKeyFlags::from_bits_retain(
        CacheKeyFlags::DISABLE_HINTING.bits()
            | (u32::from(strength) << STRENGTH_SHIFT)
            | (u32::from(gamma) << GAMMA_SHIFT)
            | if dark { DARK_BIT } else { 0 },
    )
}

/// The strength byte a shaped glyph carries in its cache key.
fn strength_of(key: &CacheKey) -> u8 {
    (key.flags.bits() >> STRENGTH_SHIFT) as u8
}

/// The text-gamma amount a shaped glyph carries in its cache key.
fn gamma_of(key: &CacheKey) -> u8 {
    (key.flags.bits() >> GAMMA_SHIFT) as u8
}

/// Whether the glyph was shaped for a dark page.
pub(crate) fn dark_of(key: &CacheKey) -> bool {
    key.flags.bits() & DARK_BIT != 0
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
    let mut curve = Curve::new();
    for (buf, ox, oy, _, _) in buffers {
        for run in buf.layout_runs() {
            // Where the cell's top edge sits above this run's baseline — the
            // placement a synthesized glyph needs so its box covers exactly
            // the cell the character was laid into.
            let cell_top = (run.line_y.round() - run.line_top) as i32;
            for glyph in run.glyphs.iter() {
                let key = glyph.physical((*ox, *oy), 1.0).cache_key;
                if swash.image_cache.contains_key(&key) {
                    continue;
                }
                // Frames, rules, bars and shades are drawn, not read from the
                // font: `boxglyph` says so first, and what it returns skips
                // both the darkening and the coverage curve below — a
                // rectangle asked for neither. The cell box comes off the
                // layout itself (the advance is snapped to one cell, the line
                // height IS the cell height), so nothing has to be plumbed in.
                // A non-finite advance saturates the cast below to `u32::MAX`
                // rather than erroring, so it is checked here as a float.
                let box_px = (glyph.w.round(), run.line_height.round());
                if let Some(image) = run.text[glyph.start..glyph.end]
                    .chars()
                    .next()
                    .filter(|_| box_px.0.is_finite() && box_px.1.is_finite())
                    .and_then(|c| {
                        crate::boxglyph::synth(c, box_px.0 as u32, box_px.1 as u32, cell_top)
                    })
                {
                    swash.image_cache.insert(key, Some(image));
                    continue;
                }
                let image = swash.get_image_uncached(font_system, key).map(|image| {
                    if image.content != SwashContent::Mask {
                        // Colour glyphs (emoji) carry their own pixels; the
                        // stem darkening and the coverage curve are both
                        // statements about alpha, so they pass through.
                        return image;
                    }
                    // The strength the ladder names is the one calibrated at
                    // body size; a smaller glyph takes proportionally less of
                    // it (see `size_scale`). The size is right here in the
                    // key — no plumbing needed to ask what it was shaped at.
                    let strength =
                        strength_at(strength_of(&key), f32::from_bits(key.font_size_bits));
                    let mut image = if strength > 0 {
                        smooth_mask(&image, strength)
                    } else {
                        image
                    };
                    curve.apply(&mut image.data, dark_of(&key), gamma_of(&key));
                    image
                });
                swash.image_cache.insert(key, image);
            }
        }
    }
}

#[cfg(test)]
#[path = "smoothing_tests.rs"]
mod tests;
