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
pub(crate) const STRAIGHT_OVER: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendState::ALPHA_BLENDING.color,
    alpha: wgpu::BlendComponent::OVER,
};

#[cfg(test)]
#[path = "blend_tests.rs"]
mod tests;
