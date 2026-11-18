//! Liquid glass: the panes as slabs of smoked glass over the desktop — the
//! iPhone's material (the user, 2026-10-07: "100% look and feel of iphone
//! liquid glass"; 2026-10-09: every theme). Like [`crate::CrtStyle`] and
//! [`crate::ModernStyle`] this is pure data; crew-render's glass pass draws
//! it.
//!
//! The window paints no wallpaper: the page is a tint at the window's
//! opacity over the desktop, and each pane's body hides more of the desktop
//! than the gaps between panes do, so the text has a calm field. A thin
//! specular rim and a broad gloss ride on top ([`crate::glass::style_for`]).
//! There was a lens too — the body blurring, saturating and bending what lay
//! behind it — until the page behind it became one flat colour and the lens
//! had nothing to bend (removed 2026-10-09).

/// How a liquid-glass window and its panes let the desktop through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidStyle {
    /// The window's opacity under this glass: how much of the desktop behind
    /// crew the wallpaper hides (1 = an opaque window). Glass is see-through
    /// (the user, 2026-10-08: "glass theme is not glassy enough, I can't see
    /// the background") — the page is a tint over the desktop.
    pub window: f32,
    /// How much of what the page leaves of the desktop a pane's slab
    /// hides on top of it (1 = a solid slab). The body is frosted glass, not
    /// a hole: the text needs a calm field, so a pane hides more of the
    /// desktop than the gaps between panes do ([`LiquidStyle::pane_cover`]).
    pub body: f32,
    /// Radius (pt) the window server blurs the desktop behind a see-through
    /// window. Small enough that what is behind crew keeps its shapes — the
    /// difference between glass and fog.
    pub desktop_blur: f32,
    /// How HOLOGRAPHIC the glass is, `0.0..=1.0` (neon, 2026-10-22: "dark
    /// glassmorphism … glowing borders … holographic effects … fine grid
    /// lines"): a lit grid behind the panes, a frame-light bleeding into each
    /// slab and an iridescent rim ([`crate::holo`]). 0 is plain liquid glass.
    pub holo: f32,
    /// How far the cards stand in RELIEF, `0.0..=1.0`: NEUMORPHISM (the
    /// user, 2026-11-12: themes with "lots of 3D effect … no border needed
    /// on pane, just an elevated card", then a neumorphism board). No frame
    /// is stroked; every card is the page's own colour, moulded out of it by
    /// two soft shadows, a light one toward the light and a dark one away
    /// from it, and the input bar is pressed INTO the page the same way
    /// ([`crate::glass::relief_glass`]). 0 is glass with a stroked frame.
    pub relief: f32,
    /// How much the relief cards are METAL, `0.0..=1.0` (the user,
    /// 2026-11-17: "actual metal look, instead of black and just some
    /// baloon in it .. some shiny look"): each plate's face is brushed
    /// (hairline grain along x), carries a broad sheen band the pointer
    /// slides, and its edge is a machined chamfer, lit on the light's side
    /// and dark on the far one. Only on a relief theme; 0 is plain relief.
    /// The face as drawn is [`metal_face`].
    pub metal: f32,
}

/// The most a metal plate's face moves from where it rests, as a fraction of
/// the way to white (the sheen) or to black (the grain's dark hairlines and
/// the far side), in sRGB. crew-render's `metal_face` draws what
/// [`metal_face`] computes; `metal_tests` holds every ink against it.
pub const METAL_SWING: f32 = 0.12;

/// Where a DARK plate rests: this far from its page toward white. A dark
/// palette's page is pinned near black by its ladder (ink at 10:1, the bright
/// white ANSI slot at 16.8:1), and steel at that depth reads as black (the
/// user, 2026-11-17: "instead of black"), so the plate stands a step lighter
/// than the page it is raised from. A pressed-in well stays at the page.
pub const METAL_REST_DARK: f32 = 0.07;

/// How much of [`METAL_SWING`] a dark plate takes toward white: the sheen
/// on a dark plate is held lower, so the quiet inks keep 3:1 under it.
pub const METAL_UP_DARK: f32 = 0.6;

/// The face of a metal plate over `page` at signed light `k` (-1..=1: the
/// grain's dark hairlines to the sheen's peak), in sRGB: what the glass
/// shader's `metal_face` draws, for the tests that floor the inks on it.
pub fn metal_face(page: (u8, u8, u8), dark: bool, k: f32) -> (u8, u8, u8) {
    let k = k.clamp(-1.0, 1.0);
    let f = |v: u8| {
        let mut c = f32::from(v) / 255.0;
        let up = if dark {
            c += (1.0 - c) * METAL_REST_DARK;
            METAL_SWING * METAL_UP_DARK
        } else {
            METAL_SWING
        };
        c += (1.0 - c) * up * k.max(0.0);
        c *= 1.0 - METAL_SWING * (-k).max(0.0);
        (c * 255.0).round() as u8
    };
    (f(page.0), f(page.1), f(page.2))
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

    /// How much of what is under it a cell's own smoke hides — a code
    /// field, an inline chip, a quiet button — drawn as a second sheet of
    /// the pane's glass, as thick as the pane's two: `1 − (1 − pane)²`. At
    /// the pane's cover alone a white desktop lifted the code field until a
    /// light palette's comments read 2.8:1 on it and its ladder (comment,
    /// code, prose) no longer fitted between field and ink (glass survey
    /// D-H1).
    pub fn cell_cover(self) -> f32 {
        let see = 1.0 - self.pane_cover();
        1.0 - see * see
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
