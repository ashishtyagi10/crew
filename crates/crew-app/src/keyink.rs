//! The ink a key wears beside its label.
//!
//! Split from [`crate::palette`] (at its line ceiling).
use crate::accentink::accent_ink;
use crate::palette::focus_color;
use ratatui::style::{Color, Modifier, Style};

/// Below this OKLCH chroma the accent has no hue to tell a key by: the Ink
/// pages' grey, or a grey the user picked.
const GREY_CHROMA: f32 = 0.03;

/// Whether `c` is a grey, so it can mark a key by lightness alone.
pub(crate) fn achromatic(c: (u8, u8, u8)) -> bool {
    crew_theme::oklch::from_srgb(c).c < GREY_CHROMA
}

/// The colour a key (or a welcome chord) is drawn in. A hued accent is the
/// key's own colour. A grey one only marks a key if it is the stronger ink:
/// on paper-light the accent is (85,85,85) beside ink at (28,28,28), so
/// `/keys` drew its keys LIGHTER than their descriptions (survey #7, loop 39)
/// and the hierarchy read upside down. There the page's ink, the darkest
/// thing it has, takes over.
pub(crate) fn key_ink() -> (u8, u8, u8) {
    let t = crew_theme::theme();
    let a = accent_ink();
    let page = t.page_bg;
    let weaker = crew_theme::contrast_ratio(a, page) < crew_theme::contrast_ratio(t.ink, page);
    match achromatic(a) && weaker {
        true => t.ink,
        false => a,
    }
}

/// A key and its label, as `/keys` and the far F-key bar set them: the key
/// in the accent, the label in ink. On a tube accent and ink are one phosphor
/// 1.1:1 apart (1.02 on amber) and the pair read as one run of text, so there
/// the key goes bold and the label steps back to `text_muted` — brightness
/// and weight, the two things a one-hue screen still has. A grey accent has
/// no hue either, so its key goes bold in [`key_ink`]: weight is then what
/// tells a key from its words.
pub fn key_label() -> (Style, Style) {
    let t = crew_theme::theme();
    let rgb = |(r, g, b): (u8, u8, u8)| Color::Rgb(r, g, b);
    match t.is_tube() {
        // `focus_accent`: the accent pushed until it clears muted on a tube.
        true => (
            Style::new().fg(focus_color()).add_modifier(Modifier::BOLD),
            Style::new().fg(rgb(t.text_muted)),
        ),
        false => {
            let key = Style::new().fg(rgb(key_ink()));
            let key = match achromatic(accent_ink()) {
                true => key.add_modifier(Modifier::BOLD),
                false => key,
            };
            (key, Style::new().fg(rgb(t.ink)))
        }
    }
}

#[cfg(test)]
#[path = "keyink_tests.rs"]
mod tests;
