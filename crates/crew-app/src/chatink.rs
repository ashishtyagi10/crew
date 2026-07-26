//! Where chat markdown gets its colours. Every helper reads the ACTIVE theme
//! at call time, so a live `/theme` switch repaints with nothing to
//! invalidate.
//!
//! Code and marker colours come from the theme's own 16-slot ANSI palette
//! rather than from new `Theme` fields. Two reasons: all 13 presets already
//! tune those slots (a new field would be 13 values to hand-maintain), and
//! single-phosphor CRT presets keep their hue for free — CRT_GREEN's "cyan"
//! is (0, 255, 200), still green, so a green tube never sprouts a foreign
//! colour. `crew-theme`'s `contrast_thresholds` test holds both slots at
//! >= 4.5:1 against the page in every preset.
//!
//! Named `chatink`, not `chatpal`: `chatpalette.rs` is the slash-command
//! palette UI and the two are easy to confuse.
use crate::chatbody::Color;

/// The code card's background: the page nudged toward the ink colour, so the
/// block reads as a card in every theme without a dedicated theme slot.
pub(crate) fn code_bg() -> Color {
    let t = crew_theme::theme();
    crate::anim::lerp_rgb(t.page_bg, t.ink, 0.08)
}

/// Code text — fenced blocks and inline spans alike.
pub(crate) fn code_fg() -> Color {
    crew_theme::theme().ansi[6]
}

/// Structural marker glyphs: list bullets/ordinals and the blockquote bar.
pub(crate) fn marker_fg() -> Color {
    crew_theme::theme().ansi[3]
}

/// Quoted prose — one step back from body text.
pub(crate) fn quote_fg() -> Color {
    crew_theme::theme().text_muted
}

/// Headings, at every level.
pub(crate) fn heading_fg() -> Color {
    crew_theme::theme().ink
}

/// Link tint: reuse the terminal pane's own URL-highlight colour (`linkhl`)
/// so a link reads the same whether it's in a pane or a chat card.
pub(crate) fn link_color() -> Color {
    crate::linkhl::LINK_FG
}
