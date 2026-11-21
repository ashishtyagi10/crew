//! A terminal drawn between two rows while its scroll glides — the app's
//! half of pixel-smooth scrolling (see [`crate::scrollglide`]).
//!
//! The pane's rows are the rows the glide has handed it; the content is
//! drawn up by the remainder, and the line that slides in under the last
//! row is the one the glide read for this frame. Both answers come from
//! [`shift`], so a pane at rest draws exactly what it always did.
use crew_render::{CellView, Paint, PaneScene};

use crate::pane::Pane;

/// How far up `p`'s content is drawn, in whole px (`ch` a row): a fractional
/// origin would raster every glyph again at a sub-pixel offset no prewarm
/// seeded. Zero when there is no line to show under it, or the remainder
/// rounds away — then the frame is the grid's own.
pub(crate) fn shift(p: &Pane, ch: f32) -> f32 {
    match p.glide.scroll.edge {
        // Under a whole row: a full row up is the next row's own frame.
        Some(_) => {
            (p.glide.scroll.residual() * ch)
                .round()
                .max(1.0 - ch.round())
                + 0.0
        }
        None => 0.0,
    }
}

/// [`Pane::art`], with the line under the viewport added while the pane is
/// drawn between two rows — only then: unshifted, nothing would clip it.
pub(crate) fn art(p: &Pane, focused: bool, cw: f32, ch: f32) -> (Vec<CellView>, Vec<Paint>) {
    let (mut cells, paint) = p.art(focused, ch / cw);
    if let Some(edge) = p
        .glide
        .scroll
        .edge
        .as_deref()
        .filter(|_| shift(p, ch) != 0.0)
    {
        cells.extend(crate::session::to_cellviews(edge));
    }
    (cells, paint)
}

/// The content scene's defaults for `p`: drawn [`shift`] px off the grid.
pub(crate) fn scene(p: &Pane, ch: f32) -> PaneScene {
    PaneScene {
        scroll_px: shift(p, ch),
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "scrollpx_tests.rs"]
mod tests;
