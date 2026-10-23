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
//! The words on its top and bottom rule stand half outside the box, where the
//! card's shadow ran through them (the `/keys` hint read as struck through on
//! glass over a light desktop). The shader keeps the shadow off them and lays
//! the page in under them, feathered as a pane title's veil (`glass.wgsl`):
//! drawn here as a box a cell high, that patch was a hard tab (2026-10-23).
use crate::corners::Corner;
use crate::quads::Quad;

/// The quads behind a pop-up at (`x`, `y`) of `w`×`h` px framed by
/// the rounded `corners`, in `color`.
#[allow(clippy::too_many_arguments)] // a pane's geometry, unbundled for the test
pub(crate) fn backdrop(
    (x, y, w, h): (f32, f32, f32, f32),
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
    vec![Quad {
        radii: [round; 4],
        ..Quad::rect(
            x + f32::from(c0) * cell_w + ix,
            y + f32::from(r0) * cell_h + iy,
            f32::from(c1 - c0) * cell_w,
            f32::from(r1 - r0) * cell_h,
            color,
        )
    }]
}

#[cfg(test)]
#[path = "popupback_tests.rs"]
mod tests;
