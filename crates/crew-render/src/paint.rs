//! Sub-cell vector paint: the layer where crew draws *pictures* instead of text.
//!
//! A [`CellView`](crate::CellView) can only ever say one glyph in one cell, so
//! every chart the app had before this layer was assembled out of block glyphs
//! — eight height levels, one column per sample. That is enough for a
//! sparkline and nothing else: a circle, an arc, a diagonal or a filled area
//! cannot be spelled in a cell grid at all.
//!
//! A [`Paint`] is one axis-aligned rectangle, addressed in **cell units** —
//! `x` counts columns from the pane's left edge, `y` counts rows from its top,
//! and both may be fractional. The scene builder multiplies by the frame's
//! cell size, so a widget composes at whatever sub-cell resolution it likes
//! without ever learning the font metrics; the same paint draws correctly
//! after a `/font` change or on a Retina rescale.
//!
//! Paint is emitted after every cell background and before the text pass, so
//! a chart sits *over* the page and *under* its own labels — the one order
//! that lets a value be written across the shape it describes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    /// Columns from the pane's left edge (fractional).
    pub x: f32,
    /// Rows from the pane's top edge (fractional).
    pub y: f32,
    /// Width in columns.
    pub w: f32,
    /// Height in rows.
    pub h: f32,
    pub color: (u8, u8, u8),
    /// `0.0..=1.0`, blended over what is already there — the quad pipeline
    /// blends alpha since this layer exists (before it, everything drawn was
    /// opaque and `REPLACE` was indistinguishable).
    pub alpha: f32,
}

impl Paint {
    /// A solid rectangle in cell units.
    pub fn solid(x: f32, y: f32, w: f32, h: f32, color: (u8, u8, u8)) -> Self {
        Self {
            x,
            y,
            w,
            h,
            color,
            alpha: 1.0,
        }
    }

    /// The same rectangle at `alpha`.
    pub fn at(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Shift by whole cells — how a widget built at its own origin is placed
    /// into a section of a larger card, matching `CellView::row += offset`.
    pub fn shifted(mut self, cols: f32, rows: f32) -> Self {
        self.x += cols;
        self.y += rows;
        self
    }

    /// Whether this rectangle would put any colour on the screen at all.
    /// Sub-pixel slivers and fully transparent paint are dropped before they
    /// reach the GPU: a chart rasterizer emits a great many of both.
    pub fn visible(&self) -> bool {
        self.w > 0.0 && self.h > 0.0 && self.alpha > 0.002
    }
}

#[cfg(test)]
#[path = "paint_tests.rs"]
mod tests;
