//! The shapes a [`super::Canvas`] can be asked for — a rectangle, a hairline,
//! a point — and the sub-pixel coverage that decides how much ink each cell
//! gets.
//!
//! Split from [`super::canvas`] for the line cap, along the line between
//! filling a region and working out what region a shape covers.
use super::canvas::{Canvas, Px};

impl Canvas {
    /// An axis-aligned rectangle in units.
    #[allow(dead_code)] // the primitive every later widget builds bars from
    /// An axis-aligned rectangle in units, at EXACT coverage.
    ///
    /// Not through [`fill`](Self::fill): a predicate sampled on a 3×3 grid
    /// answers in ninths, and the samples sit at a sixth, a half and five
    /// sixths of the pixel — so an edge in the first sixth of a pixel reads
    /// as full coverage and one in the last sixth as none. A rectangle's
    /// coverage is the overlap of two intervals and there is no reason to
    /// guess at it. This matters more now the canvas rasterizes at one pixel
    /// per DEVICE pixel: that ninth IS a pixel on the screen.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: (u8, u8, u8), alpha: f32) {
        if w <= 0.0 || h <= 0.0 || alpha <= 0.0 {
            return;
        }
        let span = |lo: f32, hi: f32, n: usize| {
            let a = ((lo * self.scale).floor() as isize).max(0) as usize;
            let b = (((hi) * self.scale).ceil() as isize).clamp(0, n as isize) as usize;
            (a.min(n), b)
        };
        let (x0, x1) = span(x, x + w, self.w);
        let (y0, y1) = span(y, y + h, self.h);
        let overlap = |i: usize, lo: f32, hi: f32| {
            let (a, b) = (i as f32 / self.scale, (i + 1) as f32 / self.scale);
            (hi.min(b) - lo.max(a)).max(0.0) * self.scale
        };
        for iy in y0..y1 {
            let cy = overlap(iy, y, y + h);
            if cy <= 0.0 {
                continue;
            }
            for ix in x0..x1 {
                let cov = cy * overlap(ix, x, x + w);
                if cov <= 0.0 {
                    continue;
                }
                let a = alpha * cov;
                let src = Px {
                    r: color.0 as f32 / 255.0 * a,
                    g: color.1 as f32 / 255.0 * a,
                    b: color.2 as f32 / 255.0 * a,
                    a,
                };
                let i = iy * self.w + ix;
                self.px[i] = self.px[i].over(src);
            }
        }
    }

    /// One canvas pixel, in units. Anything thinner than this can fall
    /// entirely between the 3×3 coverage samples and draw *nothing* — which is
    /// what happened to the NET chart's centre line, described in a module doc
    /// and invisible on screen since the day it was written.
    pub fn px(&self) -> f32 {
        1.0 / self.scale
    }

    /// A one-pixel horizontal rule from `x` for `w` units, snapped onto the
    /// pixel grid so it lands on sample points instead of between them. The
    /// thinnest mark the canvas can reliably make, and the one every widget's
    /// axis and baseline should use.
    pub fn hairline(&mut self, x: f32, y: f32, w: f32, color: (u8, u8, u8), alpha: f32) {
        let p = self.px();
        // Clamped into the grid so a baseline asked for at the box's bottom
        // edge lands on the last pixel row rather than one row past it.
        let last = (self.h as f32 - 1.0) * p;
        self.rect(
            x,
            ((y / p).floor() * p).clamp(0.0, last),
            w,
            p,
            color,
            alpha,
        );
    }

    /// Centre of pixel `(ix, iy)` in units.
    pub(crate) fn centre(&self, ix: usize, iy: usize) -> (f32, f32) {
        (
            (ix as f32 + 0.5) / self.scale,
            (iy as f32 + 0.5) / self.scale,
        )
    }

    /// Fraction of pixel `(ix, iy)` the shape covers.
    ///
    /// Two passes. The first is the 3×3 grid this has always used, and for
    /// the pixels wholly inside a shape or wholly outside it — which is very
    /// nearly all of them — that is the whole answer. A pixel whose nine
    /// samples DISAGREE is on the edge, and nine samples can only tell an
    /// edge apart in ninths: on a near-horizontal roof, where coverage slides
    /// slowly along the row, ten levels is a terrace you can see. Those
    /// pixels are re-sampled at 7×7 for fifty.
    ///
    /// The refinement is bounded by the shape's PERIMETER rather than its
    /// area, so it costs a fraction of the fill, and the coarse pass is
    /// unchanged — a mark thin enough to fall between the nine samples was
    /// invisible before this and is invisible after it, no more and no less.
    /// (Thin marks want [`fill_sdf`](Self::fill_sdf); axis-aligned ones want
    /// [`rect`](Self::rect), which computes its coverage outright.)
    pub(crate) fn coverage(&self, ix: usize, iy: usize, inside: &impl Fn(f32, f32) -> bool) -> f32 {
        let grid = |n: usize| {
            let mut hits = 0;
            for sy in 0..n {
                for sx in 0..n {
                    let px = (ix as f32 + (sx as f32 + 0.5) / n as f32) / self.scale;
                    let py = (iy as f32 + (sy as f32 + 0.5) / n as f32) / self.scale;
                    if inside(px, py) {
                        hits += 1;
                    }
                }
            }
            hits
        };
        const COARSE: usize = 3;
        const FINE: usize = 7;
        match grid(COARSE) {
            0 => 0.0,
            n if n == COARSE * COARSE => 1.0,
            _ => grid(FINE) as f32 / (FINE * FINE) as f32,
        }
    }
}
