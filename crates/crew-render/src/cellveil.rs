//! A cell background on a sheer glass window, drawn as a second sheet of the
//! pane's glass.
//!
//! Cell backgrounds are drawn opaque, which on paper is what they are. On a
//! see-through glass window the pane around them lets the desktop through
//! and they did not: a chat's code block and its inline `code` chips were
//! solid grey slabs on the glass (2026-10-09 glass survey, #1).
//!
//! A colour that is a step from the page toward the ink — the code field is
//! exactly that, by construction (`codefield::code_field`) — is a lighter
//! smoke, so it is drawn as one: its own colour, covering what is under it
//! as much as the pane's glass does ([`crew_theme::LiquidStyle::pane_cover`]).
//! The desktop tints it the way it tints the pane. Not the ink thinned to
//! land on the same colour over the smoke: over a light desktop that frosted
//! the block lighter than the glass, and the comments on it went with it.
//! A colour off that line (a selection's hue, a pasted colour, a TUI's
//! status bar in blue) is still drawn solid.

/// How far, in sRGB steps per channel, a colour may sit from the page–ink
/// line and still count as a step along it (rounding in `lerp_rgb`).
const ON_LINE: f32 = 3.0;

/// Whether `c` is a step from `page` toward `ink`: strictly between them,
/// within [`ON_LINE`] of the line.
pub(crate) fn smoke(c: (u8, u8, u8), page: (u8, u8, u8), ink: (u8, u8, u8)) -> bool {
    let f = |v: (u8, u8, u8)| [v.0 as f32, v.1 as f32, v.2 as f32];
    let (c3, p3, i3) = (f(c), f(page), f(ink));
    let d: Vec<f32> = (0..3).map(|k| i3[k] - p3[k]).collect();
    let len2: f32 = d.iter().map(|x| x * x).sum();
    if len2 < 1.0 {
        return false;
    }
    // Where `c` projects on the page→ink segment, as a fraction of it.
    let t = (0..3).map(|k| (c3[k] - p3[k]) * d[k]).sum::<f32>() / len2;
    let off = (0..3)
        .map(|k| (c3[k] - (p3[k] + t * d[k])).abs())
        .fold(0.0, f32::max);
    off <= ON_LINE && t > 0.0 && t < 1.0 && c != page && c != ink
}

/// The alpha a cell background `bg` is drawn at: the pane glass's cover for
/// a smoke on a sheer window, solid otherwise.
pub(crate) fn alpha(bg: (u8, u8, u8), sheer: bool) -> f32 {
    let t = crew_theme::theme();
    match (sheer, t.liquid) {
        (true, Some(l)) if smoke(bg, t.page_bg, t.ink) => l.pane_cover(),
        _ => 1.0,
    }
}

#[cfg(test)]
#[path = "cellveil_tests.rs"]
mod tests;
