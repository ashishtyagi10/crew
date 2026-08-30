//! The todo list's own scroll gutter: a thumb down the rightmost content
//! column, drawn only while the list is taller than the rows it has.
//!
//! A terminal pane says where it is in its buffer on the card's border
//! ([`crate::panescroll`]) — a border a drawn pane's `cells` can't reach, and
//! a reading `/todo` never had at all. A short tile showed three of six items
//! and looked exactly like a list of three: nothing said the other half was
//! below, and nothing said which half you were looking at.
//!
//! Drawn only when it has something to say, for the same reason the terminal
//! thumb is: a permanent track on a list that fits is chrome for a question
//! nobody is asking.
use crew_render::CellView;

use super::render::cell;

/// The thumb's cells over the list band: `top ..< top + lh` at column `col`,
/// given how many list rows sit above the viewport and how many there are in
/// all. Empty when everything fits.
pub(crate) fn cells(above: u16, total: u16, top: u16, lh: u16, col: u16) -> Vec<CellView> {
    if lh == 0 || total <= lh {
        return Vec::new();
    }
    let t = crew_theme::theme();
    // Proportional, and never shorter than one cell — a 2000-row list still
    // has to put something on the track.
    let span = ((lh as u32 * lh as u32) / total as u32).max(1) as u16;
    // The last row of the list is the last row of the track: a thumb that
    // stops one short of the bottom reads as "there is still more" forever.
    let travel = lh - span;
    let range = total - lh;
    let start = ((above.min(range) as u32 * travel as u32) / range as u32) as u16;
    (0..lh)
        .map(|i| {
            let on = (start..start + span).contains(&i);
            let (glyph, fg) = if on {
                ('\u{2503}', t.text_muted) // ┃
            } else {
                ('\u{2502}', t.border_normal) // │
            };
            cell(col, top + i, glyph, fg, false)
        })
        .collect()
}

#[cfg(test)]
#[path = "gutter_tests.rs"]
mod tests;
