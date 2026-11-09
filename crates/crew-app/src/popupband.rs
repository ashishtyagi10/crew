//! The rows a composer pop-up stands on, blanked beside it.
//!
//! A pop-up (`popupplace`) covers the start of the transcript rows it sits
//! on, as wide as its card. What showed to its right
//! was the rest of each of those rows — `ice-pixel canvas, and the gantt's
//! now-rule…` — half a line with its head bitten off, reading as a broken
//! render rather than as something underneath. Those rows are cleared from
//! the card's edge to the pane's, so the band is the pop-up's and the
//! transcript resumes, whole, above it.
//!
//! They are cleared UNDER the card too. Its backing starts at its top stroke
//! (`scene.rs`, the overlay sheet), half a row below the top of the band, so
//! the transcript row there showed through around the legend's letters:
//! `commands` printed over `provider: anthropic`, struck through (survey
//! H1). The bottom stroke left the descenders of the last row showing under
//! it the same way.
//!
//! The pane's own cells are dropped rather than a page-coloured sheet laid
//! over them: a sheet would be a hard stripe across a frosted pane, where
//! dropping the glyphs shows the pane's own ground.
use crate::chat::ChatPane;
use crew_render::{CellView, Paint};
use crew_term::GridSize;

/// `(top, bottom, from_col)` in the pane's content cells (`cols` × `rows`):
/// the rows `top..bottom` the card spans and the first column past its frame.
///
/// [`popupplace::above_composer`] measures the pane on this same content
/// grid, so the band's rows are the card's. The card is as wide as the pane's
/// CARD rect allows (`render.rs` asks `popup` with the rect's columns) and
/// stands on that rect's left edge, a cell left of the content — hence the
/// `cols + 2` and the column shifted back in.
///
/// [`popupplace::above_composer`]: crate::popupplace::above_composer
pub(crate) fn band(c: &ChatPane, cols: u16, rows: u16) -> Option<(u16, u16, u16)> {
    let p = c.popup(cols + 2)?;
    let bottom = rows.saturating_sub(crate::chatplace::grants(c, cols, rows).bottom);
    Some((
        bottom.saturating_sub(p.rows),
        bottom,
        p.cols.saturating_sub(1),
    ))
}

/// The chat pane's art, with the pop-up's band cleared beside the card when
/// the pop-up is showing — which is exactly when the pane is `focused` (the
/// caller's focus already excludes the input bar holding the keys).
pub(crate) fn art(
    c: &ChatPane,
    focused: bool,
    g: GridSize,
    aspect: f32,
) -> (Vec<CellView>, Vec<Paint>) {
    let (cols, rows) = (g.cols, g.rows);
    let (mut cells, paint) = crate::chatview::art(c, cols, rows, aspect);
    if let Some((top, bottom, _)) = focused.then(|| band(c, cols, rows)).flatten() {
        cells.retain(|x| !(x.row >= top && x.row < bottom));
    }
    (cells, paint)
}

#[cfg(test)]
#[path = "popupband_tests.rs"]
mod tests;
