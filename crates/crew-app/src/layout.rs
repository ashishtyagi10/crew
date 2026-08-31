#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Snap edges to whole device pixels. Scene coordinates are physical
    /// (surface) pixels end-to-end (glyphon areas prepare at `scale: 1.0`),
    /// so `round()` lands text origins and border strokes on the pixel grid —
    /// one atlas bin per glyph instead of four subpixel ones. Edges snap
    /// independently and w/h re-derive, so a boundary shared pre-snap snaps
    /// the same on both sides: no 1px gap or overlap between neighbors.
    pub fn snapped(self) -> Rect {
        let x = self.x.round();
        let y = self.y.round();
        Rect {
            x,
            y,
            w: (self.x + self.w).round() - x,
            h: (self.y + self.h).round() - y,
        }
    }
}

/// Interior cell grid of a fieldset card `w`×`h` px with one border cell per
/// side: `floor(px/cell) − 2`, min 1 per axis. The single source of the
/// rect→cells convention, shared by PTY sizing (`relayout_one`), card drawing
/// (`push_card`), and border-button hit-testing (`min_btn_rect`) so they can
/// never disagree about where a cell sits.
pub fn card_inner_cells(w: f32, h: f32, cell_w: f32, cell_h: f32) -> (u16, u16) {
    let cols = ((w / cell_w).floor() as u16).saturating_sub(2).max(1);
    let rows = ((h / cell_h).floor() as u16).saturating_sub(2).max(1);
    (cols, rows)
}

/// Pack `n` tiles into `w`x`h` offset by `(ox, oy)` as a **vertical split**:
/// the area is divided into `ceil(sqrt(n))` equal-width columns, and a column
/// is split into rows only when it must hold more than one pane. When `n`
/// isn't a multiple of the column count the surplus lands in the *earlier*
/// (left) columns, so the later columns stay full height — e.g. three panes
/// give two columns, the first split in two and the second full height.
///
/// Outer edges keep the full `gap`; interior edges take half each, so the seam
/// between two adjacent panes is one `gap` — tiles sit closer to each other
/// than to the window chrome.
pub fn pane_rects_at(n: usize, ox: f32, oy: f32, w: f32, h: f32, gap: f32) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let cols = (n as f32).sqrt().ceil() as usize;
    let base = n / cols; // rows in the shortest (right-hand) columns
    let extra = n % cols; // the first `extra` columns carry one more pane
    let tile_w = w / cols as f32;
    let half = gap / 2.0;
    let mut out = Vec::with_capacity(n);
    for c in 0..cols {
        let col_n = base + if c < extra { 1 } else { 0 };
        let tile_h = h / col_n as f32;
        let left = if c == 0 { gap } else { half };
        let right = if c == cols - 1 { gap } else { half };
        for r in 0..col_n {
            let top = if r == 0 { gap } else { half };
            let bottom = if r == col_n - 1 { gap } else { half };
            out.push(
                Rect {
                    x: ox + c as f32 * tile_w + left,
                    y: oy + r as f32 * tile_h + top,
                    w: tile_w - left - right,
                    h: tile_h - top - bottom,
                }
                .snapped(),
            );
        }
    }
    out
}

#[cfg(test)]
#[path = "layout_snap_tests.rs"]
mod snap_tests;

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
