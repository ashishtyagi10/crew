//! A raster canvas whose pixels are smaller than a cell.
//!
//! **Coordinates.** User space is *square units*: one unit is one cell
//! **width**, on both axes. A circle of radius `r` drawn here is a circle on
//! the screen — which it would not be if `y` counted rows, since a cell is
//! about twice as tall as it is wide. The canvas is told the frame's
//! `aspect` (`cell_h / cell_w`) once, and converts back to the cell units
//! [`Paint`] speaks when the drawing is done.
//!
//! **Anti-aliasing**, two ways. A shape can be an inside/outside predicate,
//! sampled on a 3×3 grid inside each pixel — ten coverage levels, enough for
//! an area fill's roof or a heat cell's edge. Or it can be a [signed
//! distance](crate::plot::sdf), where coverage is computed from the distance
//! to the edge and the ramp is continuous. Curves want the second: at the
//! size the nav draws a gauge, ten levels on a grid coarser than the screen's
//! own pixels is a staircase you can see, and a mark thinner than a canvas
//! pixel can miss all nine samples and draw *nothing*.
//!
//! **Output.** [`Canvas::paint`] run-length-merges the buffer horizontally
//! *and* vertically, so a solid region costs one rectangle rather than one per
//! pixel — a filled area chart in the sidebar comes out in the low hundreds of
//! quads, next to the ~1500 the cell backgrounds already push every frame.
use crew_render::Paint;

/// One horizontal run of identical pixels on one row: `(x, width, rgb, alpha)`.
type Run = (usize, usize, (u8, u8, u8), f32);
/// A run that is still growing downward: the run plus the row it started on.
type OpenRect = (usize, usize, (u8, u8, u8), f32, usize);

/// Straight (non-premultiplied) colour is what [`Paint`] wants, but
/// compositing wants premultiplied, so the buffer keeps premultiplied floats
/// and un-premultiplies once, at emit.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct Px {
    pub(crate) r: f32,
    pub(crate) g: f32,
    pub(crate) b: f32,
    pub(crate) a: f32,
}

impl Px {
    /// Source-over: `src` painted on top of `self`, both premultiplied.
    pub(crate) fn over(self, src: Px) -> Px {
        let k = 1.0 - src.a;
        Px {
            r: src.r + self.r * k,
            g: src.g + self.g * k,
            b: src.b + self.b * k,
            a: src.a + self.a * k,
        }
    }

    /// Back to `(rgb, alpha)`, quantized — the quantization is what lets two
    /// neighbouring pixels of "the same" colour merge into one rectangle.
    pub(crate) fn resolve(self) -> ((u8, u8, u8), f32) {
        if self.a <= 0.0 {
            return ((0, 0, 0), 0.0);
        }
        let un = |c: f32| ((c / self.a).clamp(0.0, 1.0) * 255.0).round() as u8;
        // 1/64 alpha steps: finer than the eye resolves on a chart fill, coarse
        // enough that a gradient's flat stretches collapse into single quads.
        let a = (self.a.clamp(0.0, 1.0) * 64.0).round() / 64.0;
        ((un(self.r), un(self.g), un(self.b)), a)
    }
}

pub struct Canvas {
    /// Pixel grid.
    pub(crate) w: usize,
    pub(crate) h: usize,
    /// Pixels per unit (= per cell width) on both axes; the grid is square.
    pub(crate) scale: f32,
    /// `cell_h / cell_w` — how many units tall one row is.
    pub(crate) aspect: f32,
    pub(crate) px: Vec<Px>,
}

impl Canvas {
    /// A canvas covering `cols` × `rows` cells, at `aspect = cell_h / cell_w`,
    /// rasterized at one pixel per device pixel — see
    /// [`device`](crate::plot::device).
    pub fn new(cols: u16, rows: u16, aspect: f32) -> Self {
        Self::with_sub(cols, rows, aspect, crate::plot::device::sub())
    }

    pub fn with_sub(cols: u16, rows: u16, aspect: f32, sub: usize) -> Self {
        let aspect = if aspect.is_finite() && aspect > 0.1 {
            aspect
        } else {
            2.0
        };
        let sub = sub.max(1);
        let w = (cols as usize * sub).max(1);
        let h = ((rows as f32 * aspect * sub as f32).round() as usize).max(1);
        Self {
            w,
            h,
            scale: sub as f32,
            aspect,
            px: vec![Px::default(); w * h],
        }
    }

    /// The drawing area in square units: `(width, height)`. Widgets lay out
    /// against this, never against the pixel grid.
    pub fn size(&self) -> (f32, f32) {
        (self.w as f32 / self.scale, self.h as f32 / self.scale)
    }

    /// How many units tall one cell row is — a widget that wants to line a
    /// shape up with a text row needs it.
    #[allow(dead_code)] // widgets that align a shape to a text row use it
    pub fn row_units(&self) -> f32 {
        self.aspect
    }

    /// Fill everywhere `inside` returns true, in one colour.
    pub fn fill(
        &mut self,
        bbox: (f32, f32, f32, f32),
        color: (u8, u8, u8),
        alpha: f32,
        inside: impl Fn(f32, f32) -> bool,
    ) {
        self.fill_shaded(bbox, inside, |_, _| (color, alpha));
    }

    /// Fill `inside`, taking colour and alpha per pixel — vertical gradients,
    /// value-coloured heat, a fill that fades toward a baseline.
    pub fn fill_shaded(
        &mut self,
        bbox: (f32, f32, f32, f32),
        inside: impl Fn(f32, f32) -> bool,
        shade: impl Fn(f32, f32) -> ((u8, u8, u8), f32),
    ) {
        let (bx, by, bw, bh) = bbox;
        if bw <= 0.0 || bh <= 0.0 {
            return;
        }
        let x0 = ((bx * self.scale).floor() as isize).max(0) as usize;
        let y0 = ((by * self.scale).floor() as isize).max(0) as usize;
        let x1 = (((bx + bw) * self.scale).ceil() as isize).clamp(0, self.w as isize) as usize;
        let y1 = (((by + bh) * self.scale).ceil() as isize).clamp(0, self.h as isize) as usize;
        for iy in y0..y1 {
            for ix in x0..x1 {
                let cov = self.coverage(ix, iy, &inside);
                if cov <= 0.0 {
                    continue;
                }
                let (cx, cy) = self.centre(ix, iy);
                let (rgb, a) = shade(cx, cy);
                let a = a * cov;
                if a <= 0.0 {
                    continue;
                }
                let src = Px {
                    r: rgb.0 as f32 / 255.0 * a,
                    g: rgb.1 as f32 / 255.0 * a,
                    b: rgb.2 as f32 / 255.0 * a,
                    a,
                };
                let i = iy * self.w + ix;
                self.px[i] = self.px[i].over(src);
            }
        }
    }

    /// Fill a shape given as a [signed distance](crate::plot::sdf): coverage
    /// comes off the distance analytically instead of out of a 3×3 sample
    /// grid.
    ///
    /// The sampled path snaps an edge to one of ten levels on the canvas
    /// grid, and canvas pixels are coarser than device pixels — which is the
    /// staircase visible on any arc the nav draws, and the reason a mark
    /// thinner than a canvas pixel could fall between all nine samples and
    /// draw nothing at all. A distance says how far the edge is, so it lands
    /// anywhere on a continuous ramp and a hairline comes out grey rather
    /// than absent.
    pub fn fill_sdf(
        &mut self,
        bbox: (f32, f32, f32, f32),
        color: (u8, u8, u8),
        alpha: f32,
        sd: impl Fn(f32, f32) -> f32,
    ) {
        self.fill_sdf_shaded(bbox, sd, |_, _| (color, alpha));
    }

    /// [`fill_sdf`](Self::fill_sdf), taking colour and alpha per pixel.
    pub fn fill_sdf_shaded(
        &mut self,
        bbox: (f32, f32, f32, f32),
        sd: impl Fn(f32, f32) -> f32,
        shade: impl Fn(f32, f32) -> ((u8, u8, u8), f32),
    ) {
        let (bx, by, bw, bh) = bbox;
        if bw <= 0.0 || bh <= 0.0 {
            return;
        }
        let x0 = ((bx * self.scale).floor() as isize).max(0) as usize;
        let y0 = ((by * self.scale).floor() as isize).max(0) as usize;
        let x1 = (((bx + bw) * self.scale).ceil() as isize).clamp(0, self.w as isize) as usize;
        let y1 = (((by + bh) * self.scale).ceil() as isize).clamp(0, self.h as isize) as usize;
        for iy in y0..y1 {
            for ix in x0..x1 {
                let (cx, cy) = self.centre(ix, iy);
                // Distance in units, expressed in pixels: half a pixel inside
                // the edge is fully covered, half a pixel outside is empty.
                let cov = (0.5 - sd(cx, cy) * self.scale).clamp(0.0, 1.0);
                if cov <= 0.0 {
                    continue;
                }
                let (rgb, a) = shade(cx, cy);
                let a = a * cov;
                if a <= 0.0 {
                    continue;
                }
                let src = Px {
                    r: rgb.0 as f32 / 255.0 * a,
                    g: rgb.1 as f32 / 255.0 * a,
                    b: rgb.2 as f32 / 255.0 * a,
                    a,
                };
                let i = iy * self.w + ix;
                self.px[i] = self.px[i].over(src);
            }
        }
    }

    /// The drawing as [`Paint`] rectangles in cell units, merged along both
    /// axes. Transparent pixels emit nothing.
    pub fn paint(&self) -> Vec<Paint> {
        // One pass per row builds that row's runs; a run that repeats exactly
        // on the next row grows downward instead of emitting a second quad.
        let mut out: Vec<Paint> = Vec::new();
        let mut open: Vec<OpenRect> = Vec::new();
        let px_h = 1.0 / (self.scale * self.aspect); // one pixel, in rows
        let px_w = 1.0 / self.scale; // one pixel, in columns
        for iy in 0..self.h {
            let mut runs: Vec<Run> = Vec::new();
            let mut ix = 0;
            while ix < self.w {
                let (rgb, a) = self.px[iy * self.w + ix].resolve();
                if a <= 0.0 {
                    ix += 1;
                    continue;
                }
                let start = ix;
                while ix < self.w {
                    let (r2, a2) = self.px[iy * self.w + ix].resolve();
                    if r2 != rgb || a2 != a {
                        break;
                    }
                    ix += 1;
                }
                runs.push((start, ix - start, rgb, a));
            }
            // Close every open rectangle this row does not continue, and adopt
            // the rest.
            let mut still: Vec<OpenRect> = Vec::new();
            for (x, w, rgb, a, y0) in open.drain(..) {
                if runs
                    .iter()
                    .any(|r| r.0 == x && r.1 == w && r.2 == rgb && r.3 == a)
                {
                    still.push((x, w, rgb, a, y0));
                } else {
                    out.push(Paint {
                        x: x as f32 * px_w,
                        y: y0 as f32 * px_h,
                        w: w as f32 * px_w,
                        h: (iy - y0) as f32 * px_h,
                        color: rgb,
                        alpha: a,
                    });
                }
            }
            for (x, w, rgb, a) in runs {
                if !still
                    .iter()
                    .any(|o| o.0 == x && o.1 == w && o.2 == rgb && o.3 == a)
                {
                    still.push((x, w, rgb, a, iy));
                }
            }
            open = still;
        }
        for (x, w, rgb, a, y0) in open {
            out.push(Paint {
                x: x as f32 * px_w,
                y: y0 as f32 * px_h,
                w: w as f32 * px_w,
                h: (self.h - y0) as f32 * px_h,
                color: rgb,
                alpha: a,
            });
        }
        out
    }
}

#[cfg(test)]
#[path = "canvas_tests.rs"]
mod tests;
#[cfg(test)]
mod rect_tests {
    use super::Canvas;

    /// A rectangle's coverage is the overlap of two intervals, and a canvas
    /// pixel is a SCREEN pixel now — so a partly-covered one has to carry the
    /// fraction it is actually covered by. Sampled on a 3×3 grid it carried
    /// ninths, and an edge inside the first sixth of a pixel read as full.
    #[test]
    fn a_partly_covered_pixel_carries_its_exact_fraction() {
        let mut c = Canvas::with_sub(2, 1, 1.0, 4);
        // Half of the first pixel, in x and in y: a quarter of its area.
        c.rect(0.0, 0.0, 0.125, 0.125, (255, 255, 255), 1.0);
        let quads = c.paint();
        assert_eq!(quads.len(), 1, "one partly-covered pixel: {quads:?}");
        assert!(
            (quads[0].alpha - 0.25).abs() <= 1.0 / 64.0,
            "a quarter-covered pixel reads {:.3}",
            quads[0].alpha
        );
    }

    /// …and a whole-pixel rectangle stays whole: the case every snapped rule,
    /// bar and block in crew is, and the one that must not pick up a fringe.
    #[test]
    fn a_whole_pixel_rectangle_has_no_fringe() {
        let mut c = Canvas::with_sub(2, 1, 1.0, 4);
        c.rect(0.25, 0.0, 0.25, 0.25, (255, 255, 255), 1.0);
        let quads = c.paint();
        assert_eq!(quads.len(), 1, "{quads:?}");
        assert_eq!(quads[0].alpha, 1.0, "a whole pixel picked up a fringe");
    }

    /// An edge a hair inside a pixel used to round to the whole pixel. It is
    /// a hair now, which is what an antialiased edge is for.
    #[test]
    fn an_edge_just_inside_a_pixel_is_a_hair_not_a_pixel() {
        let mut c = Canvas::with_sub(2, 1, 1.0, 4);
        c.rect(0.0, 0.0, 0.25 / 8.0, 0.25, (255, 255, 255), 1.0);
        let quads = c.paint();
        assert_eq!(quads.len(), 1, "{quads:?}");
        assert!(quads[0].alpha < 0.3, "read as {:.3}", quads[0].alpha);
        assert!(quads[0].alpha > 0.0, "an eighth of a pixel drew nothing");
    }
}
