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
use crew_render::CellView;

use crate::panecard::{put, Bar};

/// Stamp `⇡N` on the top border, ending at column `rx`, and return the next
/// free column to its left. `rx` unchanged when there is nothing to say or no
/// room to say it.
pub(crate) fn count(v: &mut Vec<CellView>, rx: u16, scroll: usize) -> u16 {
    if scroll == 0 {
        return rx;
    }
    let s = format!("\u{21e1}{scroll}");
    let w = s.chars().count() as u16;
    if rx < w {
        return rx;
    }
    let start = rx + 1 - w;
    for (i, ch) in s.chars().enumerate() {
        put(
            v,
            start + i as u16,
            0,
            ch,
            crew_theme::theme().status_fg,
            false,
        );
    }
    start.saturating_sub(2)
}

/// Rows of the right border a thumb may occupy: everything between the two
/// corners. A card shorter than this has no gutter worth drawing.
const MIN_ROWS: u16 = 5;

/// Draw the proportional thumb down the right border of a `cols`×`rows` card.
/// No-op at the bottom of the buffer, on a card with no scrollback to speak
/// of, or on one too short to read.
pub(crate) fn thumb(v: &mut Vec<CellView>, cols: u16, rows: u16, b: &Bar) {
    if b.scroll == 0 || rows < MIN_ROWS || cols < 2 {
        return;
    }
    let visible = usize::from(rows - 2);
    // How far down the buffer the top of the window sits.
    let first = b.total.saturating_sub(visible).saturating_sub(b.scroll);
    let Some((top, len)) = crate::chatscroll::thumb(b.total, visible, first) else {
        return;
    };
    let t = crew_theme::theme();
    for i in top..(top + len).min(visible) {
        put(v, cols - 1, 1 + i as u16, '\u{2503}', t.status_fg, true);
    }
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
