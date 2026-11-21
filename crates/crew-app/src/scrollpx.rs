//! A terminal drawn between two rows while its scroll glides — the app's
//! half of pixel-smooth scrolling (see [`crate::scrollglide`]).
//!
//! The pane's rows are the rows the glide has handed it; the content is
//! drawn up by the remainder, and the line that slides in under the last
//! row is the one the glide read for this frame. Both answers come from the
//! glide, so a pane at rest draws exactly what it always did.
use crew_render::{CellView, Paint, PaneScene};

use crate::pane::Pane;

/// [`Pane::art`], with the line under the viewport added while the pane is
/// drawn between two rows.
pub(crate) fn art(p: &Pane, focused: bool, aspect: f32) -> (Vec<CellView>, Vec<Paint>) {
    let (mut cells, paint) = p.art(focused, aspect);
    if let Some(edge) = &p.glide.scroll.edge {
        cells.extend(crate::session::to_cellviews(edge));
    }
    (cells, paint)
}

/// The content scene's defaults for `p`: drawn up by the glide's remainder
/// of a row, `ch` px tall, when there is a line to show under it.
pub(crate) fn scene(p: &Pane, ch: f32) -> PaneScene {
    let scroll_px = match p.glide.scroll.edge {
        Some(_) => p.glide.scroll.residual() * ch,
        None => 0.0,
    };
    PaneScene {
        scroll_px,
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "scrollpx_tests.rs"]
mod tests;
