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
    /// The window's opacity under this glass: how much of the desktop behind
    /// crew the wallpaper hides (1 = an opaque window). Glass is see-through
    /// (the user, 2026-10-08: "glass theme is not glassy enough, I can't see
    /// the background") — the page is a tint over the desktop. Opacity % in
    /// Settings can only lower it, and its floor is above glass's.
    pub window: f32,
    /// How much of what the wallpaper leaves of the desktop a pane's slab
    /// hides on top of it (1 = a solid slab). The body is frosted glass, not
    /// a hole: the text needs a calm field, so a pane hides more of the
    /// desktop than the gaps between panes do ([`LiquidStyle::pane_cover`]).
    pub body: f32,
    /// Radius (pt) the window server blurs the desktop behind a see-through
    /// window. Small enough that what is behind crew keeps its shapes — the
    /// difference between glass and fog.
    pub desktop_blur: f32,
}

/// How much of its full strength a theme's text shadow ([`crate::CrtStyle`]
/// `shade`) reaches behind the sparsest run of text: crew-render's composite
/// ramps it in over a blurred mask of the ink, which measured 0.45 under a
/// thin line of text — 0.42 of the way up its ramp. Dense text gets all of it.
pub const SHADE_UNDER_TEXT: f32 = 0.42;

impl LiquidStyle {
    /// How much of the desktop a pane hides in all: the wallpaper's share,
    /// then the slab's over what that leaves (Porter-Duff "over").
    pub fn pane_cover(self) -> f32 {
        self.body + self.window * (1.0 - self.body)
    }

    /// How much of the desktop is hidden behind a run of text, where the
    /// text shadow (`shade`, the theme's `CrtStyle::shade`) dims some of what
    /// the pane lets through: the words' own ground, which is what their
    /// colours are floored against.
    pub fn text_cover(self, shade: f32) -> f32 {
        let c = self.pane_cover();
        c + (1.0 - c) * shade * SHADE_UNDER_TEXT
    }
}

#[cfg(test)]
#[path = "liquid_tests.rs"]
mod tests;
