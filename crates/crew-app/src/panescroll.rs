//! Where a pane is in its scrollback, drawn on the card's border.
//!
//! Two readings of one fact. `⇡N` on the top border has always said how far
//! from the bottom you are; it never said how far back there *is*, so 200
//! lines up reads identically in a screenful of history and in a week of it.
//! The thumb down the right border answers the other half.
//!
//! It rides the border, not a content column, on purpose: a terminal's columns
//! belong to the program running in it, and an 80-column layout must stay 80
//! columns whether or not anyone has scrolled. It also draws only while
//! scrolled back — a permanent gutter on every card would be chrome for a
//! question nobody is asking at the bottom of the buffer.
//!
//! What is left here is the count and the colour. The thumb, the program's
//! progress bar, landmarks and search hits are drawn, in
//! [`crate::cardpaint`] and [`crate::cardmarks`].
use crew_render::CellView;

use crate::panecard::put;

/// The colour both readings wear: the theme gradient sampled at `t`, where 0
/// is the top of the buffer and 1 the live bottom.
///
/// The position is already drawn twice — as a number and as a thumb — and
/// this is the third reading, the one you get without reading anything: deep
/// in the history the marker wears `pole_a`, at the live edge `pole_b`, and
/// dragging the gutter walks it between them. It is the same gradient the
/// card's own stroke runs, so the thumb reads as part of the frame it rides
/// rather than as a widget parked on it.
///
/// Falls back to `text_muted` on a theme with no gradient to sample: an Ink
/// Reader page (2026-10-16) marks where you are in a grey of its ink, as its
/// meters do, not in a lit amber.
pub(crate) fn position_fg(t: f32) -> (u8, u8, u8) {
    crate::modernring::pole_mix(t).unwrap_or_else(|| crew_theme::theme().text_muted)
}

/// How far through the buffer the viewport's top edge sits, `0.0` at the top
/// of the scrollback and `1.0` at the live bottom. `0.5` when there is
/// nothing to be far through — a buffer with no history has no position to
/// report, and the midpoint keeps the colour off both extremes.
pub(crate) fn position(total: usize, visible: usize, scroll: usize) -> f32 {
    let range = total.saturating_sub(visible);
    if range == 0 {
        return 0.5;
    }
    1.0 - (scroll.min(range) as f32 / range as f32)
}

/// Stamp `⇡N` (or a file's `⇣N` — see [`crate::panehome`]) on the top
/// border, ending at column `rx` and never reaching left of `min_col` (the
/// legend's floor — see [`crate::panecard`]), and return the next free column
/// to its left. `rx` unchanged when there is nothing to say or no room.
pub(crate) fn count(
    v: &mut Vec<CellView>,
    rx: u16,
    min_col: u16,
    away: (usize, char),
    t: f32,
) -> u16 {
    let (n, arrow) = away;
    if n == 0 {
        return rx;
    }
    let s = format!("{arrow}{n}");
    let w = s.chars().count() as u16;
    if rx < w || rx + 1 - w < min_col {
        return rx;
    }
    let start = rx + 1 - w;
    let fg = position_fg(t);
    for (i, ch) in s.chars().enumerate() {
        put(v, start + i as u16, 0, ch, fg, false);
    }
    start.saturating_sub(2)
}

/// The scroll offset that puts the top of the window on the line a pointer
/// `frac` of the way down the gutter is pointing at: 0.0 is the top of the
/// buffer, 1.0 the live bottom. Returns lines back from the bottom, which is
/// what `display_offset` counts in.
pub(crate) fn offset_at(total: usize, visible: usize, frac: f32) -> usize {
    let range = total.saturating_sub(visible);
    let first = (frac.clamp(0.0, 1.0) * range as f32).round() as usize;
    range - first.min(range)
}

#[cfg(test)]
#[path = "panescroll_tests.rs"]
mod tests;
