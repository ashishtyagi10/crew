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
/// Falls back to `status_fg` — what both wore before — on a theme with no
/// gradient to sample.
fn position_fg(t: f32) -> (u8, u8, u8) {
    crate::modernring::pole_mix(t).unwrap_or(crew_theme::theme().status_fg)
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

/// Stamp `⇡N` on the top border, ending at column `rx`, and return the next
/// free column to its left. `rx` unchanged when there is nothing to say or no
/// room to say it.
pub(crate) fn count(v: &mut Vec<CellView>, rx: u16, scroll: usize, t: f32) -> u16 {
    if scroll == 0 {
        return rx;
    }
    let s = format!("\u{21e1}{scroll}");
    let w = s.chars().count() as u16;
    if rx < w {
        return rx;
    }
    let start = rx + 1 - w;
    let fg = position_fg(t);
    for (i, ch) in s.chars().enumerate() {
        put(v, start + i as u16, 0, ch, fg, false);
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
    // A shell's gutter is a scrollback affordance — nothing behind you, no
    // gutter. A document's is where you ARE in it, which is a question worth
    // answering at the top of the file too.
    if (b.scroll == 0 && !b.doc) || rows < MIN_ROWS || cols < 2 {
        return;
    }
    let visible = usize::from(rows - 2);
    // How far down the buffer the top of the window sits.
    let first = b.total.saturating_sub(visible).saturating_sub(b.scroll);
    let Some((top, len)) = crate::chatscroll::thumb(b.total, visible, first) else {
        return;
    };
    let fg = position_fg(position(b.total, visible, b.scroll));
    for i in top..(top + len).min(visible) {
        put(v, cols - 1, 1 + i as u16, '\u{2503}', fg, true);
    }
}

/// Landmark ticks down the right border: one dim mark per rendered row worth
/// jumping to (`]` / `[`), placed proportionally. Drawn BEFORE the thumb, so
/// the landmark you are sitting on is covered by the thumb rather than the
/// other way round — the thumb is where you are, and that answer wins.
///
/// Ticks are deduplicated by row: a long document has more headings than the
/// gutter has cells, and two landmarks in one cell is one mark, not two.
pub(crate) fn ticks(v: &mut Vec<CellView>, cols: u16, rows: u16, b: &Bar) {
    if b.ticks.is_empty() || rows < MIN_ROWS || cols < 2 || b.total == 0 {
        return;
    }
    let inner = usize::from(rows - 2);
    let fg = crew_theme::theme().legend_off;
    let mut drawn: Vec<u16> = Vec::new();
    for &row in b.ticks {
        let at = (row.min(b.total.saturating_sub(1)) * inner) / b.total.max(1);
        let y = 1 + at.min(inner.saturating_sub(1)) as u16;
        if drawn.contains(&y) {
            continue;
        }
        drawn.push(y);
        put(v, cols - 1, y, '\u{2508}', fg, false);
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
