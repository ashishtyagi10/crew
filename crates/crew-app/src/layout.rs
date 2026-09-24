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

/// The space around cards, per axis: `x`/`y` between two cards' rects, and
/// `mx`/`my` between a card's rect and the window's edge.
///
/// Spaced by rects, a gutter is not what the eye measures. It measures the
/// LINES, and a frame's `│` sits half a cell WIDTH inside its rect while its
/// `─` sits half a cell HEIGHT inside — twice as far — so one gap between
/// rects drew a seam twice as wide between stacked cards as between side-by-
/// side ones. [`Gutter::between_strokes`] solves for the rect gaps that put
/// one gutter between every pair of lines and one margin at every window
/// edge. A bare `f32` is the old even gap, all four the same.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gutter {
    pub x: f32,
    pub y: f32,
    pub mx: f32,
    pub my: f32,
}

impl From<f32> for Gutter {
    fn from(g: f32) -> Self {
        Self {
            x: g,
            y: g,
            mx: g,
            my: g,
        }
    }
}

impl Gutter {
    /// Rect gaps for frames whose strokes sit `inset` px inside their rects:
    /// the stroke-to-stroke gutter is `gap` plus ONE side inset, on BOTH
    /// axes, and the window margin, line to edge, is that same gutter.
    /// (`gap` plus both insets — a whole cell of air on top of the gap — was
    /// the seam two side-by-side cards once had; as the one gutter
    /// everywhere it read as cards drifting apart.) Neighbouring rects then
    /// overlap slightly ... and the px they overlap on are the empty halves
    /// beyond each card's rule.
    pub fn between_strokes(gap: f32, inset: (f32, f32)) -> Self {
        let (ix, iy) = inset;
        let g = gap + ix;
        Self {
            x: g - 2.0 * ix,
            y: g - 2.0 * iy,
            mx: g - ix,
            my: g - iy,
        }
    }
}

/// Pack `n` tiles into `w`x`h` offset by `(ox, oy)` as a **vertical split**:
/// the area is divided into `ceil(sqrt(n))` equal-width columns, and a column
/// is split into rows only when it must hold more than one pane. When `n`
/// isn't a multiple of the column count the surplus lands in the *earlier*
/// (left) columns, so the later columns stay full height — e.g. three panes
/// give two columns, the first split in two and the second full height.
///
/// Outer edges keep the margin (`mx`/`my`); interior edges take half the
/// gutter each, so the seam between two adjacent panes is one gutter.
pub fn pane_rects_at(
    n: usize,
    ox: f32,
    oy: f32,
    w: f32,
    h: f32,
    gap: impl Into<Gutter>,
) -> Vec<Rect> {
    let g: Gutter = gap.into();
    if n == 0 {
        return Vec::new();
    }
    let cols = (n as f32).sqrt().ceil() as usize;
    let base = n / cols; // rows in the shortest (right-hand) columns
    let extra = n % cols; // the first `extra` columns carry one more pane
    let tile_w = w / cols as f32;
    let (hx, hy) = (g.x / 2.0, g.y / 2.0);
    let mut out = Vec::with_capacity(n);
    for c in 0..cols {
        let col_n = base + if c < extra { 1 } else { 0 };
        let tile_h = h / col_n as f32;
        let left = if c == 0 { g.mx } else { hx };
        let right = if c == cols - 1 { g.mx } else { hx };
        for r in 0..col_n {
            let top = if r == 0 { g.my } else { hy };
            let bottom = if r == col_n - 1 { g.my } else { hy };
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

#[cfg(test)]
#[path = "layout_gutter_tests.rs"]
mod gutter_tests;
