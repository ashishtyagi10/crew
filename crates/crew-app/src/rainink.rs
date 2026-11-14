//! The colour crew's rain falls in: the welcome's field and a busy smith
//! pane's code line.
//!
//! The rain was `text_muted` on every palette, so sixteen of the twenty
//! welcomes were the same grey drizzle on a different page and only the two
//! tubes rained in a colour of their own (theme loop, 2026-11-14). It now
//! falls in the palette's accent hue, held at `text_muted`'s lightness so it
//! is exactly as quiet as before: ambience, never louder than the name it
//! falls around.
use crew_theme::oklch::from_srgb;
use crew_theme::Theme;

/// The most chroma the rain keeps: a tint of the accent, not the accent. A
/// neon accent at full chroma made the field the loudest thing on the card.
const CHROMA: f32 = 0.11;

/// The rain's lit colour on `t` under `accent`. An Ink Reader page stays
/// ink (print has one colour) and a tube keeps its phosphor, which is its
/// accent already.
pub(crate) fn rain_ink(t: &Theme, accent: (u8, u8, u8)) -> (u8, u8, u8) {
    if t.is_print() || t.is_tube() {
        return t.text_muted;
    }
    let (a, m) = (from_srgb(accent), from_srgb(t.text_muted));
    a.with_l(m.l).with_c(a.c.min(CHROMA)).to_srgb()
}

/// [`rain_ink`] on the theme and accent in force.
pub(crate) fn current() -> (u8, u8, u8) {
    rain_ink(crew_theme::theme(), crate::palette::accent())
}

#[cfg(test)]
#[path = "rainink_tests.rs"]
mod tests;
