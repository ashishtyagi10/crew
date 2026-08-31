//! Where the menu card's non-label ink comes from — descriptions, chords and
//! section titles, in every picker `cmdmenu` draws (the slash palette, the
//! attach popup, the model picker, `/todo`'s tag menu, every value
//! suggestion).
//!
//! It was one constant, `(120, 130, 140)`, compiled into `cmdmenu`. A fixed
//! blue-grey is a colour the palette never got to decide: it is nearly the
//! page on the darkest presets, and on the single-phosphor tubes it is not
//! even the right HUE — a blue-grey word on a green screen is the one thing
//! a phosphor cannot draw. The theme already has a muted role tuned per
//! preset; this floors it so a preset with a quiet `text_muted` still hands
//! back something a description can be read in.
use crew_theme::{contrast, readable, theme};

/// Descriptions and chords: quieter than the label, still text you read.
pub(crate) fn desc() -> (u8, u8, u8) {
    let t = theme();
    readable::against(t.text_muted, t.page_bg, contrast::mark_floor())
}

/// A ratatui colour for the same, since the menu list is laid into a `Buffer`.
pub(crate) fn desc_color() -> ratatui::style::Color {
    let (r, g, b) = desc();
    ratatui::style::Color::Rgb(r, g, b)
}

#[cfg(test)]
#[path = "menuink_tests.rs"]
mod tests;
