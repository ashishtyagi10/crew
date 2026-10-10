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
//! The pane's own cells are dropped rather than a page-coloured sheet laid
//! over them: a sheet would be a hard stripe across a frosted pane, where
//! dropping the glyphs shows the pane's own ground.
use crate::chat::ChatPane;
use crew_render::{CellView, Paint};
use crew_term::GridSize;

/// `(top, bottom, from_col)` in the pane's content cells (`cols` × `rows`):
/// the rows `top..bottom` the card spans and the first column past its frame.
///
/// [`popupplace::above_composer`] measures the pane by its CARD rect — the
/// content and the frame's cell either side — and stands the pop-up on that
/// rect's left edge, while the content is drawn one cell in. So the card's
/// geometry is taken rect-sized and handed back shifted into the content.
///
/// [`popupplace::above_composer`]: crate::popupplace::above_composer
pub(crate) fn band(c: &ChatPane, cols: u16, rows: u16) -> Option<(u16, u16, u16)> {
    let (rc, rr) = (cols + 2, rows + 2);
    let p = c.popup(rc)?;
    let g = crate::chatplace::grants(c, rc, rr);
    let bottom = rr.saturating_sub(g.bottom).saturating_sub(1);
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
    if let Some((top, bottom, from)) = focused.then(|| band(c, cols, rows)).flatten() {
        cells.retain(|x| !(x.row >= top && x.row < bottom && x.col >= from));
    }
    (cells, paint)
}

#[cfg(test)]
#[path = "popupband_tests.rs"]
mod tests;
