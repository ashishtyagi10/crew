//! The one blend state every straight-alpha pipeline draws with.
//!
//! `BlendState::ALPHA_BLENDING` blends the ALPHA channel with the colour's
//! factors too: `a = sa·sa + da·(1 − sa)`. On an opaque page that is invisible
//! (the compositor ignores alpha), but on a sheer window it pulls the alpha
//! DOWN wherever anything partly covers a pixel — a half-covered frame edge
//! over a 0.85 page lands at 0.675, more see-through than the bare glass
//! beside it. The frosted desktop then shows through the anti-aliased rim of
//! every frame line and glyph, and the borders read as broken.
//!
//! Alpha composites with Porter-Duff "over" instead — `a = sa + da·(1 − sa)`,
//! never below what was there — while colour keeps straight-alpha blending.
// Every scene pipeline now blends `PREMUL_OVER` (the scene is stored
// premultiplied since v0.26.9); this stays as the reference the tests hold it
// to.
#[cfg(test)]
pub(crate) const STRAIGHT_OVER: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendState::ALPHA_BLENDING.color,
    alpha: wgpu::BlendComponent::OVER,
};

/// [`STRAIGHT_OVER`] for a shader that hands over PREMULTIPLIED colour: the
/// same result for the same pixel, but the colour need not fit under its own
/// alpha first. The glass needs that: a see-through slab clearer than its
/// own frost has to say "this much page, gone" and "this much frost" at once,
/// and straight colour would have to divide by an alpha smaller than the
/// frost — clamping the frost greyer (see `glass.wgsl`'s see-through).
pub(crate) const PREMUL_OVER: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent::OVER,
    alpha: wgpu::BlendComponent::OVER,
};

#[cfg(test)]
#[path = "blend_tests.rs"]
mod tests;
