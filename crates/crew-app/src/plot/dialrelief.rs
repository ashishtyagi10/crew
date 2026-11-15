//! A dial's face on a RELIEF theme (neumorphism): moulded out of the card,
//! not laid on it.
//!
//! Everywhere else the face is a faint plate under a stroked bezel. On a
//! soft page that bezel was the hardest line left on a card and the plate a
//! flat grey disc (the open list of 2026-11-14: "gauge dials are flat
//! discs"). Here the face is the card's own colour and is told from it the
//! way the card is told from the page: a light shadow thrown toward the
//! light and a dark one the other way, so the dial reads as a knob raised
//! off the card. No bezel is stroked; the two shadows end the face.
use crate::plot::Canvas;

/// Where the light comes from: `LIGHT` in crew-render's `glass.wgsl`, so a
/// dial is lit from the same side as the card it stands on.
const LIGHT: (f32, f32) = (-0.45, -0.89);
/// How far each shadow is thrown, as a fraction of the face's radius, and
/// how far it fades out past its disc, in throws.
const THROW: f32 = 0.07;
const FADE: f32 = 1.6;

/// One shadow: `color` at up to `alpha` on a disc of radius `r` centred on
/// `at`, fading to nothing `blur` units past its edge.
fn shadow(c: &mut Canvas, at: (f32, f32), r: f32, blur: f32, color: (u8, u8, u8), alpha: f32) {
    let reach = r + blur;
    let bbox = (at.0 - reach, at.1 - reach, 2.0 * reach, 2.0 * reach);
    let past = move |x: f32, y: f32| ((x - at.0).hypot(y - at.1) - r).max(0.0);
    c.fill_sdf_shaded(
        bbox,
        move |x, y| past(x, y) - blur,
        move |x, y| (color, alpha * (1.0 - past(x, y) / blur).clamp(0.0, 1.0)),
    );
}

/// Draw the moulded face of a dial of radius `r` at `centre`: the light
/// shadow, the dark one, then the face over both in the page's colour.
pub fn mould(c: &mut Canvas, centre: (f32, f32), r: f32) {
    let g = crew_theme::glass_style();
    let px = c.px();
    let throw = (THROW * r).max(1.5 * px);
    let blur = FADE * throw;
    let n = LIGHT.0.hypot(LIGHT.1);
    let off = (LIGHT.0 / n * throw, LIGHT.1 / n * throw);
    let away = (centre.0 - off.0, centre.1 - off.1);
    let toward = (centre.0 + off.0, centre.1 + off.1);
    // The light one first: what it leaks round the far side is the weaker
    // of the two there, and the dark one goes over it.
    shadow(c, toward, r, blur, g.highlight, g.highlight_alpha);
    shadow(c, away, r, blur, (0, 0, 0), g.shadow_alpha);
    let bbox = (centre.0 - r, centre.1 - r, 2.0 * r, 2.0 * r);
    let page = crew_theme::theme().page_bg;
    c.fill_sdf(bbox, page, 1.0, move |x, y| {
        (x - centre.0).hypot(y - centre.1) - r
    });
}

#[cfg(test)]
#[path = "dialrelief_tests.rs"]
mod tests;
