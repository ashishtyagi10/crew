//! How fine the drawn widgets are rasterized: one canvas pixel per DEVICE
//! pixel, whatever the display and font size make that.
//!
//! [`Canvas`](super::Canvas) works in square units — one unit is one cell
//! width — and has to pick a raster resolution for them. It picked a constant
//! four, which was chosen when a cell was about eight device pixels across:
//! **every drawn widget in crew was rasterized at half the screen's
//! resolution and blown up.** On a Retina display, where a cell is sixteen
//! device pixels, it was a quarter. Coverage antialiasing cannot rescue that
//! — the smallest thing the canvas can address is the block, so a dial's
//! needle and a chart's edge step in blocks however carefully their coverage
//! is computed. Three surfaces had already noticed and hard-coded their own
//! 8, 12 and 16 locally; the rest were still at four.
//!
//! The frame knows the cell size — it is what every pane is laid out against
//! — so it publishes it here once per frame and the canvas reads it. That is
//! one global rather than a cell width threaded through every widget's
//! signature, and it is the same shape as the theme: something the whole
//! frame agrees on, set before anything draws.
use std::sync::atomic::{AtomicU32, Ordering};

/// The last cell width the frame reported, in device pixels, as bits.
/// Zero until the renderer has a real cell size (headless tests, first
/// frame), where [`sub`] falls back to [`FALLBACK`].
static CELL_W: AtomicU32 = AtomicU32::new(0);

/// Canvas pixels per cell width before the frame has said otherwise. Eight
/// is a cell at crew's default font size on a 1× display.
pub const FALLBACK: usize = 8;

/// Never coarser than this, so a tiny font still gets a usable raster, and
/// never finer, so a full-pane chart on a large Retina cell cannot run away
/// with the quad budget. (A canvas is run-length merged, so its cost grows
/// with a shape's PERIMETER rather than its area — but the perimeter grows
/// with this too.)
const FLOOR: usize = 4;
const CEIL: usize = 20;

/// Publish this frame's cell width, in device pixels.
pub fn set_cell_w(cell_w: f32) {
    let px = if cell_w.is_finite() && cell_w >= 1.0 {
        cell_w.round() as u32
    } else {
        0
    };
    CELL_W.store(px, Ordering::Relaxed);
}

/// Canvas pixels per cell width for this frame — one per device pixel,
/// clamped.
pub fn sub() -> usize {
    match CELL_W.load(Ordering::Relaxed) as usize {
        0 => FALLBACK,
        px => px.clamp(FLOOR, CEIL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point: the raster follows the device, so the same widget is
    /// drawn at 8 canvas pixels per column on a 1× display and 16 on a
    /// Retina one — never at a constant that is right for neither.
    #[test]
    fn the_raster_follows_the_cell() {
        set_cell_w(8.0);
        assert_eq!(sub(), 8);
        set_cell_w(16.0);
        assert_eq!(sub(), 16, "a Retina cell gets a Retina raster");
        set_cell_w(0.0);
        assert_eq!(sub(), FALLBACK, "no frame yet is not a zero-wide canvas");
    }

    #[test]
    fn absurd_cells_are_clamped_rather_than_believed() {
        set_cell_w(1.0);
        assert_eq!(sub(), FLOOR);
        set_cell_w(400.0);
        assert_eq!(sub(), CEIL);
        set_cell_w(f32::NAN);
        assert_eq!(sub(), FALLBACK);
        set_cell_w(8.0);
    }
}
