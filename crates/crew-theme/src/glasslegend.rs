//! What a legend stands on when the glass is see-through, and the ink that
//! reads there.
//!
//! A legend sits ON a card's rule, half over the pane and half over the gap.
//! On see-through glass neither half is the page: both are the desktop under
//! a frost, and the desktop is anything — so a title floored against
//! `page_bg` read 6:1 in the palette and 2:1 on a dark desktop, where light
//! glass turns mid-grey (the user, 2026-10-09: "hard to see title of the
//! panels … proper color should be chosen, like we have in iphone").
//!
//! The iPhone keeps a label legible on its glass two ways: it thickens the
//! material under the words, and it gives them a label colour made for that
//! material — near-black on light glass, white on dark. Crew cannot see the
//! desktop (the window server blurs it, crew never gets the pixels), so it
//! does both for the worst desktop either way: a frost veil behind every run
//! of words on a rule ([`LEGEND_FROST`], drawn by crew-render's glass pass),
//! and ink pushed away from that frost until it reads over a black desktop
//! AND a white one ([`legible`]).
use crate::glass::{style_for, GlassLevel};
use crate::{contrast_ratio, oklch, Theme};

/// How much of the body's see-through the frost behind a legend takes away
/// (`0..=1`): the slab there hides `1 − (1 − body)(1 − LEGEND_FROST)` of what
/// the window leaves. Must match `VEIL_FROST` in crew-render's `glass.wgsl`.
pub const LEGEND_FROST: f32 = 0.6;

/// The floor for a quiet legend on glass: AAA, not the page's AA — a label
/// on glass is the iPhone's near-black (or white), never a grey, and thin
/// strokes render lighter than their colour measures.
pub const LABEL_FLOOR: f32 = 7.0;
/// The focused pane's legend: a full step past the quiet ones, so focus
/// reads as the darkest (on night glass, brightest) words on the rules.
pub const FOCUSED_LABEL_FLOOR: f32 = 10.0;

/// How far one step walks the ink's lightness (OKLCH L).
const STEP: f32 = 0.01;

/// How much of the desktop the glass hides behind a legend, all told.
fn legend_cover(body: f32, window: f32) -> f32 {
    let body = 1.0 - (1.0 - body) * (1.0 - LEGEND_FROST);
    body + window * (1.0 - body)
}

/// What a legend on `t`'s glass stands on over a black desktop and over a
/// white one, at every frost level — the grounds its ink has to read on.
/// Empty for a palette that is not liquid glass.
pub fn grounds(t: &Theme) -> Vec<(u8, u8, u8)> {
    t.liquid
        .map_or_else(Vec::new, |l| grounds_at(t, legend_cover(l.body, l.window)))
}

/// What text inside a pane stands on: the body alone, no veil.
pub fn pane_grounds(t: &Theme) -> Vec<(u8, u8, u8)> {
    t.liquid
        .map_or_else(Vec::new, |l| grounds_at(t, l.pane_cover()))
}

/// The glass's frost over a black desktop and a white one, where it hides
/// `cover` of the desktop. The body's top is the worst of its ramp either
/// way: the least frost.
fn grounds_at(t: &Theme, cover: f32) -> Vec<(u8, u8, u8)> {
    let mut out = Vec::new();
    for level in [GlassLevel::Low, GlassLevel::Medium, GlassLevel::High] {
        let g = style_for(t).scaled_by(level.liquid_scale());
        let frost = |p: u8, q: u8| f32::from(p) + (f32::from(q) - f32::from(p)) * g.alpha_top;
        let body = (
            frost(t.page_bg.0, g.tint.0),
            frost(t.page_bg.1, g.tint.1),
            frost(t.page_bg.2, g.tint.2),
        );
        for desk in [0.0_f32, 255.0] {
            let shown = |c: f32| (c * cover + desk * (1.0 - cover)).round().clamp(0.0, 255.0) as u8;
            out.push((shown(body.0), shown(body.1), shown(body.2)));
        }
    }
    out
}

/// The worst contrast `fg` makes on any of `grounds`.
pub fn worst(fg: (u8, u8, u8), grounds: &[(u8, u8, u8)]) -> f32 {
    grounds
        .iter()
        .map(|&g| contrast_ratio(fg, g))
        .fold(f32::INFINITY, f32::min)
}

/// `want` as a legend on `t`'s glass: kept at its hue and chroma, walked
/// toward the glass's own label pole — darker on light glass, lighter on
/// night glass — until it clears `floor` over every desktop (see
/// [`grounds`]). Unchanged on a palette that is not glass, or once it
/// clears; the best it reached when the hue tops out first.
pub fn legible(t: &Theme, want: (u8, u8, u8), floor: f32) -> (u8, u8, u8) {
    legible_on(t, want, floor, &grounds(t))
}

/// [`legible`] over any set of `grounds` (none: unchanged).
pub fn legible_on(
    t: &Theme,
    want: (u8, u8, u8),
    floor: f32,
    grounds: &[(u8, u8, u8)],
) -> (u8, u8, u8) {
    if grounds.is_empty() || worst(want, grounds) >= floor {
        return want;
    }
    let c = oklch::from_srgb(want);
    let dir = if t.dark { 1.0 } else { -1.0 };
    let (mut best, mut best_r) = (want, worst(want, grounds));
    let mut l = c.l;
    while (0.0..=1.0).contains(&l) {
        l += dir * STEP;
        let rgb = c.with_l(l.clamp(0.0, 1.0)).to_srgb();
        let r = worst(rgb, grounds);
        if r > best_r {
            (best, best_r) = (rgb, r);
        }
        if r >= floor {
            return rgb;
        }
    }
    best
}

#[cfg(test)]
#[path = "glasslegend_tests.rs"]
mod tests;
