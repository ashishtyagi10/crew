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
struct Px {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Px {
    /// Source-over: `src` painted on top of `self`, both premultiplied.
    fn over(self, src: Px) -> Px {
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
    fn resolve(self) -> ((u8, u8, u8), f32) {
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
    w: usize,
    h: usize,
    /// Pixels per unit (= per cell width) on both axes; the grid is square.
    scale: f32,
    /// `cell_h / cell_w` — how many units tall one row is.
    aspect: f32,
    px: Vec<Px>,
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

    /// An axis-aligned rectangle in units.
    #[allow(dead_code)] // the primitive every later widget builds bars from
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: (u8, u8, u8), alpha: f32) {
        self.fill((x, y, w, h), color, alpha, |px, py| {
            px >= x && px < x + w && py >= y && py < y + h
        });
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
    fn centre(&self, ix: usize, iy: usize) -> (f32, f32) {
        (
            (ix as f32 + 0.5) / self.scale,
            (iy as f32 + 0.5) / self.scale,
        )
    }

    /// Fraction of pixel `(ix, iy)` the shape covers, from a 3×3 sample grid.
    fn coverage(&self, ix: usize, iy: usize, inside: &impl Fn(f32, f32) -> bool) -> f32 {
        const S: usize = 3;
        let mut hits = 0;
        for sy in 0..S {
            for sx in 0..S {
                let px = (ix as f32 + (sx as f32 + 0.5) / S as f32) / self.scale;
                let py = (iy as f32 + (sy as f32 + 0.5) / S as f32) / self.scale;
                if inside(px, py) {
                    hits += 1;
                }
            }
        }
        hits as f32 / (S * S) as f32
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
mod tests {
    use super::Canvas;

    /// Total painted area, in square units — the invariant every shape test
    /// below leans on. Alpha counts: a half-covered edge pixel is half a
    /// pixel of area, which is what anti-aliasing means.
    fn painted_area(c: &Canvas) -> f32 {
        c.paint()
            .iter()
            .map(|p| p.w * p.h * c.row_units() * p.alpha)
            .sum()
    }

    #[test]
    fn user_space_is_square_so_a_circle_is_round() {
        // Two cells across, one row down, at a 2:1 cell — the canvas is 2
        // units wide and 2 units tall, not 2 × 1.
        let c = Canvas::new(2, 1, 2.0);
        let (w, h) = c.size();
        assert_eq!((w, h), (2.0, 2.0));
    }

    #[test]
    fn a_filled_rectangle_covers_exactly_its_area() {
        let mut c = Canvas::new(4, 2, 2.0);
        c.rect(1.0, 1.0, 2.0, 1.5, (255, 0, 0), 1.0);
        let a = painted_area(&c);
        assert!((a - 3.0).abs() < 0.02, "3 square units painted, got {a}");
    }

    #[test]
    fn a_disc_covers_pi_r_squared_with_anti_aliased_edges() {
        let mut c = Canvas::with_sub(6, 3, 2.0, 8);
        let (cx, cy, r) = (3.0, 3.0, 2.0);
        c.fill(
            (cx - r, cy - r, 2.0 * r, 2.0 * r),
            (0, 255, 0),
            1.0,
            |x, y| (x - cx).powi(2) + (y - cy).powi(2) <= r * r,
        );
        let a = painted_area(&c);
        let want = std::f32::consts::PI * r * r;
        assert!(
            (a - want).abs() / want < 0.02,
            "disc area {a} within 2% of {want}"
        );
        // The edge is graded, not binary: a hard-edged raster would put every
        // pixel at alpha 1.0 and stair-step at these sizes.
        let graded = c.paint().iter().filter(|p| p.alpha < 0.95).count();
        assert!(graded > 8, "anti-aliased edge pixels: {graded}");
    }

    /// The point of the distance path: an edge that lands between two canvas
    /// pixels comes out as a partial one, and a mark thinner than a pixel
    /// comes out *grey* rather than missing. The sampled path cannot do
    /// either — nine samples on a grid give nine chances to miss.
    #[test]
    fn a_distance_fill_grades_edges_the_sample_grid_would_snap_or_miss() {
        let (cx, cy, r) = (3.0, 3.0, 2.0);
        let bbox = (cx - r, cy - r, 2.0 * r, 2.0 * r);
        let disc = move |x: f32, y: f32| super::super::sdf::disc((x, y), (cx, cy), r);
        let levels = |c: &Canvas| {
            let mut steps: Vec<f32> = c
                .paint()
                .iter()
                .map(|p| p.alpha)
                .filter(|a| *a < 0.99)
                .collect();
            steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
            steps.dedup();
            steps.len()
        };

        let mut sdf = Canvas::new(6, 3, 2.0);
        sdf.fill_sdf(bbox, (0, 255, 0), 1.0, disc);
        let a = painted_area(&sdf);
        let want = std::f32::consts::PI * r * r;
        assert!((a - want).abs() / want < 0.02, "disc area {a} vs {want}");

        // The same disc, sampled: nine samples can only ever land on ten
        // levels, and the same circle's edge lands on a handful of them.
        let mut sampled = Canvas::new(6, 3, 2.0);
        sampled.fill(bbox, (0, 255, 0), 1.0, move |x, y| disc(x, y) <= 0.0);
        assert!(
            levels(&sdf) > levels(&sampled),
            "distance edge {} levels vs sampled {}",
            levels(&sdf),
            levels(&sampled)
        );
    }

    /// A hairline thinner than a canvas pixel: the sample grid draws nothing
    /// unless it happens to straddle a sample row, the distance field always
    /// draws it, dimmed in proportion to how thin it is.
    #[test]
    fn a_sub_pixel_line_survives_the_distance_path() {
        let thin = |use_sdf: bool| {
            let mut c = Canvas::new(8, 2, 2.0);
            // A tenth of a pixel wide, deliberately placed off the sample rows.
            let (y, half) = (1.507, 0.05 * c.px());
            if use_sdf {
                c.fill_sdf(
                    (0.0, y - 0.5, 8.0, 1.0),
                    (255, 255, 255),
                    1.0,
                    move |_, py| (py - y).abs() - half,
                );
            } else {
                c.fill(
                    (0.0, y - 0.5, 8.0, 1.0),
                    (255, 255, 255),
                    1.0,
                    move |_, py| (py - y).abs() <= half,
                );
            }
            painted_area(&c)
        };
        assert_eq!(thin(false), 0.0, "the sample grid misses it entirely");
        assert!(thin(true) > 0.0, "the distance field keeps it");
    }

    #[test]
    fn paint_merges_runs_in_both_directions() {
        let mut c = Canvas::new(8, 4, 2.0);
        let (w, h) = c.size();
        c.rect(0.0, 0.0, w, h, (1, 2, 3), 1.0);
        // A solid fill is ONE rectangle — not one per pixel row, and not one
        // per pixel. Without the merge a full-pane chart would push tens of
        // thousands of quads a frame.
        assert_eq!(c.paint().len(), 1);
        let p = c.paint()[0];
        assert!((p.w - 8.0).abs() < 1e-3 && (p.h - 4.0).abs() < 1e-3);
    }

    #[test]
    fn nothing_drawn_paints_nothing() {
        assert!(Canvas::new(10, 4, 2.0).paint().is_empty());
    }

    #[test]
    fn paint_stays_inside_the_canvas_however_far_the_shape_runs() {
        let mut c = Canvas::new(3, 2, 2.0);
        c.rect(-5.0, -5.0, 100.0, 100.0, (7, 7, 7), 1.0);
        for p in c.paint() {
            assert!(p.x >= 0.0 && p.y >= 0.0);
            assert!(p.x + p.w <= 3.0 + 1e-3, "column overflow: {p:?}");
            assert!(p.y + p.h <= 2.0 + 1e-3, "row overflow: {p:?}");
        }
    }

    #[test]
    fn translucent_paint_composites_rather_than_replaces() {
        let mut c = Canvas::new(2, 1, 2.0);
        c.rect(0.0, 0.0, 2.0, 2.0, (0, 0, 0), 1.0);
        c.rect(0.0, 0.0, 2.0, 2.0, (255, 255, 255), 0.5);
        let p = c.paint();
        assert_eq!(p.len(), 1);
        // Half white over black reads mid-grey and fully opaque — the layer
        // below is *covered*, not discarded.
        assert!((p[0].alpha - 1.0).abs() < 1e-3);
        assert!((110..=145).contains(&p[0].color.0), "got {:?}", p[0].color);
    }

    #[test]
    fn a_shaded_fill_varies_across_the_shape() {
        let mut c = Canvas::new(4, 2, 2.0);
        let (w, h) = c.size();
        c.fill_shaded(
            (0.0, 0.0, w, h),
            |_, _| true,
            |_, y| ((255, 255, 255), (y / h).clamp(0.0, 1.0)),
        );
        let alphas: Vec<f32> = c.paint().iter().map(|p| p.alpha).collect();
        let lo = alphas.iter().cloned().fold(f32::MAX, f32::min);
        let hi = alphas.iter().cloned().fold(0.0f32, f32::max);
        assert!(hi > 0.9 && lo < 0.2, "gradient spans alpha {lo}..{hi}");
    }

    #[test]
    fn the_pixel_grid_follows_the_cell_aspect() {
        // A taller cell means more pixel rows for the same cell count, so the
        // shapes drawn in it keep their screen proportions.
        let tall = Canvas::new(4, 2, 3.0);
        let wide = Canvas::new(4, 2, 1.0);
        assert_eq!(tall.size().0, wide.size().0);
        assert!(tall.size().1 > wide.size().1);
        assert_eq!(wide.size().0, 4.0);
        assert_eq!(tall.w, 4 * crate::plot::device::sub());
    }
}
