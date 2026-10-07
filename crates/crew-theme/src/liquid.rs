//! Liquid glass: the panes as slabs of real glass over a wallpaper — the
//! iPhone's material (the user, 2026-10-07: "100% look and feel of iphone
//! liquid glass"). Like [`crate::CrtStyle`] and [`crate::ModernStyle`] this is
//! pure data; crew-render's glass pass does the optics.
//!
//! What makes the material read as glass rather than a tinted sheet is that
//! it shows what is BEHIND it, bent: the body is the wallpaper blurred,
//! saturated and then frosted with the glass tint so text reads on it, and toward the rim the glass thickens into a
//! lens that pulls the wallpaper just outside the edge in under it, its
//! colours splitting a little as a prism's do. A thin specular rim and a
//! broad gloss ride on top ([`crate::glass::style_for`]).

/// The optics of a liquid-glass card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidStyle {
    /// How far (px) the lens at the rim reaches out for what it shows: at the
    /// very edge the glass shows the wallpaper this far outside the card,
    /// falling to nothing [`LiquidStyle::bevel`] px in.
    pub refract: f32,
    /// How deep (px) the rim's lens runs into the card.
    pub bevel: f32,
    /// Radius (px) of the frost: the blur the body lays over the wallpaper.
    pub blur: f32,
    /// How far the lens splits the colours: red reaches this fraction
    /// further than green, blue this much less.
    pub dispersion: f32,
    /// How much clearer the rim's lens is than the body, `0..=1`: the body's
    /// frost (`GlassStyle`'s tint at its fill alpha) thins by this much at the
    /// very edge, so the bent wallpaper shows there bright and clean while
    /// the field under the text stays calm.
    pub clear_rim: f32,
    /// How much the glass saturates what it shows (1 = as is): the material
    /// makes colour richer, not greyer, behind it.
    pub vibrance: f32,
}
