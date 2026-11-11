//! CoreText-style glyph smoothing for the swash raster path.
//!
//! Ghostty, Warp, kitty and Terminal.app all rasterize through CoreText,
//! which never hints and (with "font smoothing" on) widens strokes by a
//! fraction of a pixel — Apple's stem darkening. swash hints by default and
//! draws exact outlines, so the same font reads thinner and rougher in crew
//! than in any native terminal. This module closes that gap:
//!
//! - every text run carries [`CacheKeyFlags::DISABLE_HINTING`] (CoreText
//!   never hints, and unhinted outlines are what subpixel binning expects),
//! - the smoothing strength rides in the cache key's spare high bits, so a
//!   `/smooth` change mints new keys and stale atlas entries simply age out,
//! - [`presmooth`] rasterizes every glyph a frame needs *before* glyphon
//!   does, with its outline grown by [`grow_px`] ([`crate::outlineink`]),
//!   and pre-fills `SwashCache.image_cache` (a public field) — glyphon's own
//!   `get_image` then hits our entry.
//!
//! # The widening is done to the outline, not to the bitmap
//!
//! Reported (2026-11-11): "fonts are not smooth enough". Measured against a
//! CoreText rendering of the same line, the glyphs were the right weight and
//! had almost no edge left: on a dark page 2% of inked pixels sat below 40%
//! coverage, where CoreText keeps 20% there. The weight had been bought by
//! pushing the antialiased rim toward solid — a kernel that deepened covered
//! pixels, then a coverage curve that lifted them again — and a rim with two
//! levels in it is a staircase.
//!
//! CoreText gets the same weight by moving the outline out and rasterizing
//! THAT, so every edge keeps its whole ramp. So does crew now. With no grow
//! and no curve, swash's coverage matches CoreText's unsmoothed output to
//! within half a percent of ink, level for level; growing the outline by
//! [`GROW_AT_MEDIUM`] lands on its smoothed output the same way.
//!
//! The flags still carry [`crate::textgamma`]'s amount and the run's
//! polarity: the curve is opt-in now, and the polarity also decides how far
//! the outline grows.
use glyphon::cosmic_text::{CacheKey, CacheKeyFlags, SwashCache, SwashContent};
use glyphon::FontSystem;

use crate::scene::PaneBuffer;
use crate::textgamma::Curve;

/// Default smoothing strength (0–255): `medium`, the strength at which the
/// grown outline matches CoreText's own font smoothing.
pub const DEFAULT_SMOOTH: u8 = 70;

/// How much wider, in pixels, a stroke gets at [`DEFAULT_SMOOTH`] on a dark page
/// at [`REFERENCE_PX`]. Measured, not chosen: JetBrains Mono Medium, white on
/// near-black, against CoreText with font smoothing on — this grow gives the
/// same count of inked pixels (within 2%), the same share of them solid, and
/// the same spread across the levels between.
const GROW_AT_MEDIUM: f32 = 0.64;

/// The glyph size, in physical pixels, that figure was measured at: 12 pt
/// on a Retina display.
const REFERENCE_PX: f32 = 24.0;

/// Where the grow stops following the size, per polarity `(dark, bright)`.
/// CoreText's widening is a fixed share of the glyph up to a size and a
/// fixed distance past it: light ink gains 27–33% from 10 px to 24 px and
/// then tails off (16% at 48 px), dark ink gains 14–15% all the way to 48.
const GROW_CAP_PX: (f32, f32) = (24.0, 48.0);

/// The share of the grow a bright page takes. CoreText widens dark ink on a
/// bright ground about half as far: dark strokes already read heavier than
/// light ones of the same width.
const BRIGHT_SHARE: f32 = 0.5;

/// How much wider a glyph's strokes are drawn, in pixels, for a `/smooth`
/// strength at a glyph size of `px` physical pixels.
pub(crate) fn grow_px(strength: u8, px: f32, dark: bool) -> f32 {
    let (cap, share) = match dark {
        true => (GROW_CAP_PX.0, 1.0),
        false => (GROW_CAP_PX.1, BRIGHT_SHARE),
    };
    let level = f32::from(strength) / f32::from(DEFAULT_SMOOTH);
    level * GROW_AT_MEDIUM * share * px.clamp(0.0, cap) / REFERENCE_PX
}

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
    for (buf, ox, oy, _) in crate::scriptrun::areas(buffers) {
        let base = crate::gridbase::baseline(buf);
        for run in buf.layout_runs() {
            // Where the cell's top edge sits above the baseline glyphon draws
            // this run on — the placement a synthesized glyph needs so its
            // box covers exactly the cell the character was laid into. The
            // grid's baseline, not the run's: the image is cached per key,
            // and the key is the same on every row.
            let cell_top = (crate::gridbase::line_y(&run, base).round() - run.line_top) as i32;
            for glyph in run.glyphs.iter() {
                let key = glyph.physical((ox, oy), 1.0).cache_key;
                if swash.image_cache.contains_key(&key) {
                    continue;
                }
                // Frames, rules, bars and shades are drawn, not read from the
                // font: `boxglyph` says so first, and what it returns skips
                // both the widening and the coverage curve below — a
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
                // The size is right here in the key — no plumbing needed to
                // ask what the glyph was shaped at.
                let px = f32::from_bits(key.font_size_bits);
                let grow = grow_px(strength_of(&key), px, dark_of(&key));
                let raw = match grow > 0.0 {
                    true => crate::outlineink::render(font_system, key, grow),
                    false => swash.get_image_uncached(font_system, key),
                };
                let image = raw.map(|mut image| {
                    // Colour glyphs (emoji) carry their own pixels; the
                    // coverage curve is a statement about alpha.
                    if image.content == SwashContent::Mask {
                        curve.apply(&mut image.data, dark_of(&key), gamma_of(&key));
                    }
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
