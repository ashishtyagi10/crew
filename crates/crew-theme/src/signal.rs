//! Derives the attention colour from the status colour, so the two markers a
//! pane can wear are not the same marker.
//!
//! ## What was wrong
//!
//! `status_fg` and `bell` are different roles. `status_fg` is progress — the
//! git dirty dot, the input-bar status, gauge fills, the swarm view.
//! `bell` is *needs you*: the pane attention glyphs (`!` rang, `⚑` matched,
//! `✓` agent done, `⊗` exited, `?` waiting) and every ERROR line in the log.
//!
//! They shipped as the **same colour**:
//!
//! | theme | Δ status_fg → bell |
//! |---|---|
//! | paper-dark | **0.000** |
//! | sepia-dark | **0.000** |
//! | sepia-light | 0.014 |
//! | paper-light | 0.025 |
//! | crt-green | 0.015 |
//! | crt-amber | 0.025 |
//! | crt-blue | 0.032 |
//! | nebula | 0.147 |
//! | blossom | 0.318 |
//!
//! One 8-bit code is Δ 0.0015 and a *visible* step is Δ 0.027, so on the two
//! paper darks these are literally one colour, and on four more they are
//! within a step of it. A pane that finished and a pane that is merely busy
//! looked the same in the strip.
//!
//! The two that got it right say what the rule should be. `nebula` and
//! `blossom` put status on the theme's own accent hue and broke `bell` away
//! to a warm ~50° orange — an alarm reads warm, and it must not be the colour
//! already spoken for by progress.
//!
//! ## The tubes are exempt, and that is not a cop-out
//!
//! A phosphor tube has ONE hue; that is the entire theme. Rotating `crt-green`'s
//! bell to orange would separate the markers and destroy the palette — the
//! exact mistake [the ramp's docs](crate::ramp) record from deriving ink from
//! the page. The ANSI work hit the same wall and settled it the same way:
//! colour themes separate by hue, phosphor tubes separate by brightness,
//! because spreading a tube by hue "would have made them legible and destroyed
//! them". A tube has no brightness left to spend here either — `crt-green`'s
//! status already sits at L 0.92 against an ink of L 0.87.
//!
//! So on a tube the separation is the one a real terminal used: the attention
//! glyph **blinks** (`Attention::visible`), and the colour does not have to
//! carry the signal alone.
use crate::oklch;

/// Minimum perceptual distance between `status_fg` and `bell` on a coloured
/// palette. A visible step is Δ 0.027 and a full rung of the text hierarchy is
/// Δ 0.10 — this sits at **three visible steps**, deliberately short of the
/// rung, because the rung is not available: these markers live at L 0.84 on
/// the paper darks, and sRGB simply does not hold enough chroma up there for
/// two hues to get a rung apart without one of them going quiet. Holding out
/// for 0.10 would mean making the alarm markedly dimmer or brighter than the
/// status beside it, which trades a real problem for a worse one. Measured
/// ceiling at status loudness across the coloured palettes: 0.079–0.088.
pub const FLOOR: f32 = 0.075;

/// The attention colour a palette should carry, given its status colour.
///
/// Returns `declared` untouched when it already clears [`FLOOR`] — the point
/// is to separate the four palettes that never did, not to re-tune the two
/// that did it properly. Otherwise walks from `status` toward the hottest
/// in-gamut [`ALARM_HUE`] at the same lightness (so the alarm stays exactly as
/// loud as the status it replaces) and stops at the first step that clears
/// the floor.
///
/// Tubes are the caller's business: pass them through, see the module docs.
pub fn alarm(
    page: (u8, u8, u8),
    status: (u8, u8, u8),
    alert: (u8, u8, u8),
    declared: (u8, u8, u8),
) -> (u8, u8, u8) {
    if oklch::distance(status, declared) >= FLOOR {
        return declared;
    }
    // Hue and chroma come from the palette's own bright-red slot — already
    // derived, gamut-checked and in-family, so nothing here invents a colour.
    // The target contrast is the STATUS's own, so the alarm is exactly as loud
    // as the marker it has to be told apart from; louder would separate them
    // by brightness instead, which is the tubes' answer, not a colour page's.
    let a = oklch::from_srgb(alert);
    oklch::solve_for_contrast(
        page,
        a.h,
        a.c,
        crate::contrast_ratio(status, page),
        oklch::Toward::for_page(page),
    )
}

/// The attention colour palette `id` should ship — [`alarm`] with the tube
/// exemption applied. This is the function the presets are held to; `alarm`
/// itself is the colour maths underneath.
pub fn alarm_for(id: crate::ThemeId) -> (u8, u8, u8) {
    let t = id.theme();
    if id.is_crt() {
        return t.bell;
    }
    alarm(t.page_bg, t.status_fg, t.ansi[9], t.bell)
}

#[cfg(test)]
#[path = "signal_tests.rs"]
mod tests;
