//! Heatmap: a grid of cells shaded by magnitude — the shape of *when*.
//!
//! A series says how much and a bar chart says how much per bucket; neither
//! says which hours of which days the work happened in. Seven rows of
//! twenty-four squares does, at a glance, in the space of a paragraph.
//!
//! Shading is by rank against the grid's own peak, not against an absolute
//! scale: a quiet week and a busy one both have to read, and the eye compares
//! within a picture, not between pictures.
use crate::plot::Canvas;

/// A cell's colour, given its value's share of the peak (`0.0..=1.0`).
pub type Shade<'a> = &'a dyn Fn(f32) -> ((u8, u8, u8), f32);

/// Draw `values` (`rows` × `cols` of them, row-major) as a grid filling
/// `(x, y, w, h)` in canvas units, with `gap` units of air between cells.
///
/// An all-zero grid still draws: every cell takes the shade of zero, so an
/// idle week reads as an empty grid rather than as a missing widget.
pub fn draw(
    c: &mut Canvas,
    rect: (f32, f32, f32, f32),
    values: &[u64],
    rows: usize,
    cols: usize,
    gap: f32,
    shade: Shade,
) {
    let (x, y, w, h) = rect;
    if rows == 0 || cols == 0 || w <= 0.0 || h <= 0.0 {
        return;
    }
    let peak = values.iter().copied().max().unwrap_or(0).max(1) as f32;
    let cw = w / cols as f32;
    let ch = h / rows as f32;
    // Cells are square-ish by construction: the caller sizes the rect, and a
    // gap eats the same amount from both axes so the squares stay square.
    let (bw, bh) = ((cw - gap).max(0.02), (ch - gap).max(0.02));
    for r in 0..rows {
        for k in 0..cols {
            let v = values.get(r * cols + k).copied().unwrap_or(0);
            let (color, alpha) = shade(v as f32 / peak);
            if alpha <= 0.0 {
                continue;
            }
            let (bx, by) = (x + k as f32 * cw, y + r as f32 * ch);
            // Rounded corners at this size would cost more than they show;
            // the gap already separates the cells.
            c.rect(bx, by, bw, bh, color, alpha);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::draw;
    use crate::plot::Canvas;

    fn shade(t: f32) -> ((u8, u8, u8), f32) {
        ((0, 200, 120), 0.08 + 0.92 * t)
    }

    fn grid(values: &[u64], rows: usize, cols: usize) -> Canvas {
        let mut c = Canvas::with_sub(24, 4, 2.0, 8);
        let (w, h) = c.size();
        draw(&mut c, (0.0, 0.0, w, h), values, rows, cols, 0.15, &shade);
        c
    }

    /// The strongest alpha painted at `(x, y)` in canvas units — a cell is
    /// several rectangles once its edges are anti-aliased, so tests read the
    /// grid by sampling it rather than by counting quads.
    fn alpha_at(c: &Canvas, x: f32, y: f32) -> f32 {
        c.paint()
            .iter()
            .filter(|p| {
                let (py, ph) = (p.y * c.row_units(), p.h * c.row_units());
                x >= p.x && x < p.x + p.w && y >= py && y < py + ph
            })
            .map(|p| p.alpha)
            .fold(0.0f32, f32::max)
    }

    /// Centre of cell `(row, col)` of an `rows`×`cols` grid on this canvas.
    fn centre(c: &Canvas, rows: usize, cols: usize, r: usize, k: usize) -> (f32, f32) {
        let (w, h) = c.size();
        (
            (k as f32 + 0.5) * w / cols as f32,
            (r as f32 + 0.5) * h / rows as f32,
        )
    }

    #[test]
    fn a_hotter_cell_is_drawn_stronger() {
        let c = grid(&[0, 1, 5, 10], 1, 4);
        let a: Vec<f32> = (0..4)
            .map(|k| {
                let (x, y) = centre(&c, 1, 4, 0, k);
                alpha_at(&c, x, y)
            })
            .collect();
        assert!(
            a.windows(2).all(|w| w[0] <= w[1] + 1e-3),
            "alpha rises with value: {a:?}"
        );
        assert!(a[3] - a[0] > 0.5, "and by a lot: {a:?}");
    }

    #[test]
    fn shading_is_relative_to_the_grids_own_peak() {
        // A quiet week and a busy one draw the same picture: the eye compares
        // within a heatmap, never between two of them.
        let quiet = grid(&[0, 1, 2, 4], 1, 4);
        let busy = grid(&[0, 1000, 2000, 4000], 1, 4);
        let sample = |c: &Canvas| -> Vec<f32> {
            (0..4)
                .map(|k| {
                    let (x, y) = centre(c, 1, 4, 0, k);
                    alpha_at(c, x, y)
                })
                .collect()
        };
        assert_eq!(sample(&quiet), sample(&busy));
    }

    #[test]
    fn an_idle_grid_still_draws_its_cells() {
        // An idle week reads as an empty grid, not as a missing widget.
        let c = grid(&[0; 12], 3, 4);
        for r in 0..3 {
            for k in 0..4 {
                let (x, y) = centre(&c, 3, 4, r, k);
                assert!(alpha_at(&c, x, y) > 0.05, "cell ({r},{k}) is drawn");
            }
        }
    }

    #[test]
    fn there_is_air_between_the_cells() {
        let c = grid(&[3; 24], 2, 12);
        let (w, h) = c.size();
        // On the boundary between two columns, nothing is painted.
        let x = w / 12.0;
        let y = h / 4.0;
        assert!(alpha_at(&c, x - 0.02, y) < 0.02, "the gap is empty");
        // …while a hair either side of it is inside a cell.
        assert!(alpha_at(&c, x + 0.1, y) > 0.05);
    }

    #[test]
    fn every_cell_stays_inside_the_rect() {
        let c = grid(&[3; 24], 2, 12);
        let (w, h) = c.size();
        for p in c.paint() {
            assert!(p.x >= -1e-3 && p.x + p.w <= w + 1e-3, "{p:?}");
            let y = p.y * c.row_units();
            assert!(y >= -1e-3 && y + p.h * c.row_units() <= h + 1e-3, "{p:?}");
        }
    }

    #[test]
    fn a_short_values_slice_leaves_the_rest_at_zero() {
        // Fewer readings than cells is normal (a week that started on
        // Wednesday); the missing ones are cold, not absent.
        let c = grid(&[9, 9], 1, 4);
        let (x0, y0) = centre(&c, 1, 4, 0, 0);
        let (x3, y3) = centre(&c, 1, 4, 0, 3);
        assert!(alpha_at(&c, x0, y0) > 0.9, "the readings are hot");
        let cold = alpha_at(&c, x3, y3);
        assert!((0.05..0.2).contains(&cold), "the rest are cold: {cold}");
    }
}
