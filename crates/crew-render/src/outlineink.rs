//! Stem darkening where CoreText does it: on the outline, before it is
//! rasterized.
//!
//! cosmic-text's own rasterizer draws the outline exactly as the font gives
//! it. This is the same call with one thing added — the outline is grown by
//! a fraction of a pixel first — so a heavier glyph is still an exact
//! coverage mask, with the whole soft ramp on every edge.
use std::cell::RefCell;

use glyphon::cosmic_text::{CacheKey, CacheKeyFlags, SwashImage};
use glyphon::FontSystem;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::{Angle, Format, Transform, Vector};

thread_local! {
    static CONTEXT: RefCell<ScaleContext> = RefCell::new(ScaleContext::new());
}

const WGHT: swash::Tag = swash::Tag::from_be_bytes(*b"wght");

/// Rasterize `key`'s glyph with every stroke `grow` pixels wider: half of
/// it on each side of a stem, all of it on top of a bar.
pub(crate) fn render(fs: &mut FontSystem, key: CacheKey, grow: f32) -> Option<SwashImage> {
    let font = fs.get_font(key.font_id, key.font_weight)?;
    let face = font.as_swash();
    CONTEXT.with_borrow_mut(|context| {
        let mut scaler = context
            .builder(face)
            .size(f32::from_bits(key.font_size_bits))
            .hint(!key.flags.contains(CacheKeyFlags::DISABLE_HINTING));
        if let Some(axis) = face.variations().find_by_tag(WGHT) {
            let value = f32::from(key.font_weight.0).clamp(axis.min_value(), axis.max_value());
            scaler = scaler.variations(std::iter::once(swash::Setting { tag: WGHT, value }));
        }
        let mut scaler = scaler.build();
        // swash's embolden moves every edge out by its strength and then the
        // whole outline up and right by the same: a stroke comes back two
        // strengths wider with its left and bottom edges where they were. So
        // half the grow is the strength. Sideways that half comes back off
        // the offset, which centres a stem on where the font put it. Upward
        // it stays, as it does in CoreText: a bar grows from its top, and
        // every glyph's foot stays exactly on the baseline it sat on.
        let side = grow / 2.0;
        let offset = Vector::new(key.x_bin.as_float() - side, key.y_bin.as_float());
        let skew = key
            .flags
            .contains(CacheKeyFlags::FAKE_ITALIC)
            .then(|| Transform::skew(Angle::from_degrees(14.0), Angle::from_degrees(0.0)));
        Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(offset)
        .transform(skew)
        .embolden(side)
        .render(&mut scaler, key.glyph_id)
    })
}

#[cfg(test)]
#[path = "outlineink_tests.rs"]
mod tests;
