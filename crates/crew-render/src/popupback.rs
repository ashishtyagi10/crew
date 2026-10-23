//! A pop-up's backdrop: a solid box in the page's colour, drawn before its
//! cells in the overlay pass, so nothing behind shows through.
//!
//! It spans the FRAME, stroke to stroke and bent with its corners, as a pane's
//! sheet does: out to the scene's rect it stood half a cell past the frame on
//! every side (and a margin column more on a composer pop-up's right), a dark
//! rim round the white line that read, on the glass, as a card set off-centre
//! in a bigger black one (glass survey C#4). A scene with no rounded frame
//! keeps its rect.
//!
//! And behind the words on its top and bottom rule, a cell-high patch more:
//! those words stand on the rule, half of them outside the box, and there the
//! card's own shadow ran through them. On glass over a light desktop the
//! `/keys` hint under its card read as struck through (UI survey 2026-10-23).
//! A base pane's sheet has a frosted veil there; a pop-up has no sheet, so
//! the backdrop reaches out instead, and the shadow stops at it (`glass.wgsl`).
use crate::cellgrid::CellView;
use crate::corners::Corner;
use crate::quads::Quad;

/// The quads behind a pop-up at (`x`, `y`) of `w`×`h` px whose `cells` hold
/// the rounded `corners`, in `color`.
#[allow(clippy::too_many_arguments)] // a pane's geometry, unbundled for the test
pub(crate) fn backdrop(
    (x, y, w, h): (f32, f32, f32, f32),
    cells: &[CellView],
    corners: &[Corner],
    (cell_w, cell_h): (f32, f32),
    (ix, iy): (f32, f32),
    round: f32,
    color: [f32; 4],
) -> Vec<Quad> {
    let Some((c0, r0, c1, r1)) = crate::corners::frame_box(corners) else {
        return vec![Quad {
            radii: [round + ix; 4],
            ..Quad::rect(x, y, w, h, color)
        }];
    };
    let mut out = vec![Quad {
        radii: [round; 4],
        ..Quad::rect(
            x + f32::from(c0) * cell_w + ix,
            y + f32::from(r0) * cell_h + iy,
            f32::from(c1 - c0) * cell_w,
            f32::from(r1 - r0) * cell_h,
            color,
        )
    }];
    let cols = usize::from(c1) + 1;
    for row in [r0, r1] {
        let runs = crate::notch::runs(cells, row, cols);
        for (a, b) in runs.into_iter().filter(|&(a, _)| a > usize::from(c0)) {
            out.push(Quad {
                radii: [(cell_h * 0.3).min(round); 4],
                ..Quad::rect(
                    x + a as f32 * cell_w,
                    y + f32::from(row) * cell_h,
                    (b - a) as f32 * cell_w,
                    cell_h,
                    color,
                )
            });
        }
    }
    out
}

#[cfg(test)]
#[path = "popupback_tests.rs"]
mod tests;
