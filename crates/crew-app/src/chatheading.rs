//! The badge before, and the rule under, a top-level heading in a chat
//! card. An `h1` is the one title a message has: it leads with an accent
//! segment carrying the heading mark, and it is underlined — as wide as its
//! row, not the card, so the rule reads as the heading's own rather than as
//! a divider between two messages. Muted, like every rule the card draws.
use crate::chatbody::{plain, CardCell, CardLine, Color};

/// Columns the h1 badge and the space after it take on the heading's first
/// row: the badge is one mark wide (`segment::width` of one cell, 5) plus
/// the gap. `md::layout` wraps an h1 this much narrower so the badge never
/// pushes the title's last word onto a row of its own.
pub(crate) const BADGE_W: usize = 6;

/// The columns a heading of `level` wraps at on a `cols`-wide card: an h1
/// leaves room for its badge, every other level takes the full width.
pub(crate) fn wrap_cols(level: u8, cols: usize) -> usize {
    if level == 1 {
        cols.saturating_sub(BADGE_W).max(1)
    } else {
        cols
    }
}

/// The h1 badge: the heading mark (`#`, or the hashtag icon on a Nerd
/// Font) on an accent block, then one space in `fg` before the title. The
/// title itself keeps the accent INK from `chatspan::heading_fg`; the badge
/// is the same accent as a block, so mark and title read as one thing.
pub(crate) fn badge_cells(fg: Color) -> Vec<CardCell> {
    let accent = crate::palette::accent();
    let mark = crate::glyphs::pick(crate::glyphs::Glyph::Hash);
    let badge = crate::segment::badge(
        mark,
        crate::segment::page_ink(accent),
        accent,
        crate::segment::Caps::BOTH,
    );
    let mut cells = crate::segment::to_card(&badge);
    cells.push(plain(' ', fg, false));
    cells
}

/// The rule row for the heading row `above`: one indent cell, then `─` for
/// every display column the heading's text covers.
pub(crate) fn rule_row(above: &CardLine, muted: Color) -> CardLine {
    let width: usize = above
        .iter()
        .skip(1)
        .map(|c| crate::chatwidth::char_w(c.c))
        .sum();
    std::iter::once(plain(' ', muted, false))
        .chain((0..width).map(|_| plain('\u{2500}', muted, false)))
        .collect()
}

#[cfg(test)]
#[path = "chatheading_tests.rs"]
mod tests;
