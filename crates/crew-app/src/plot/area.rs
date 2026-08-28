//! Area chart: a smooth curve through a series with the space beneath it
//! filled by a fade to the baseline.
//!
//! The glyph sparkline this replaces had eight height levels and one sample
//! per column — a CPU sitting at 30% and one sitting at 37% drew the identical
//! row, and every rise was a staircase. Here the curve is interpolated
//! (Catmull–Rom) and rasterized with coverage, so the shape carries the
//! reading, and the fill under it says *how much* at a glance in a way an
//! outline never does.
use crate::plot::Canvas;

/// Line thickness in units (a unit is one cell width) — about a device pixel
/// and a half at the default font size, the weight the box-drawing frames use.
const STROKE: f32 = 0.2;

/// Alpha at the curve and at the baseline: the fill has to read as *volume*
/// without competing with the text above it, which is the whole reason it
/// fades instead of filling flat.
const FILL_TOP: f32 = 0.38;
const FILL_BOTTOM: f32 = 0.04;

/// Which end of the fill carries the weight.
///
/// A chart standing on the bottom of its own box wants the weight at the
/// curve, where the shape is. A chart hanging off a shared axis — the two
/// halves of the NET twin — wants it at the axis instead: fading *toward* the
/// line leaves a pale gap either side of it, and the two halves stop reading
/// as one chart about one thing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub at_curve: f32,
    pub at_base: f32,
    /// Whether to draw the curve itself and the dot on its newest reading.
    /// A chart you *read* wants both. A chart drawn as a backdrop under text
    /// wants neither: the fill is texture and the eye passes over it, but a
    /// stroke is a line, and a line crossing a word is a scribble however
    /// faint it is.
    pub outline: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            at_curve: FILL_TOP,
            at_base: FILL_BOTTOM,
            outline: true,
        }
    }
}

impl Style {
    /// The weight at the baseline instead of the curve.
    pub fn anchored() -> Self {
        Self {
            at_curve: 0.08,
            at_base: 0.42,
            ..Self::default()
        }
    }

    /// Fill only, weighted at the floor: a backdrop for text drawn over it.
    pub fn wash() -> Self {
        Self {
            at_curve: 0.30,
            at_base: 0.02,
            outline: false,
        }
    }
}

/// Draw `samples` (each `0.0..=1.0`, oldest first) as an area chart filling
/// `(x, y, w, h)` in canvas units. The newest sample lands on the right edge —
/// the chart scrolls left as readings arrive, like every other live series in
/// the sidebar.
pub fn draw(c: &mut Canvas, rect: (f32, f32, f32, f32), samples: &[f32], color: (u8, u8, u8)) {
    draw_styled(c, rect, samples, color, Style::default());
}

/// [`draw`] with the fill's weight placed explicitly — see [`Style`].
pub fn draw_styled(
    c: &mut Canvas,
    rect: (f32, f32, f32, f32),
    samples: &[f32],
    color: (u8, u8, u8),
    style: Style,
) {
    let (x, y, w, h) = rect;
    if w <= 0.0 || h <= 0.0 || samples.is_empty() {
        return;
    }
    // Baseline sits half a stroke inside the box so the rule is not clipped.
    let base = y + h - STROKE * 0.5;
    let top = y + STROKE * 0.5;
    let span = (base - top).max(0.001);
    let curve = |px: f32| -> f32 {
        let t = ((px - x) / w).clamp(0.0, 1.0);
        base - value_at(samples, t).clamp(0.0, 1.0) * span
    };

    // The fill: everything between the curve and the baseline, shaded from
    // FILL_TOP at the curve down to FILL_BOTTOM at the floor.
    c.fill_shaded(
        (x, y, w, h),
        |px, py| px >= x && px <= x + w && py >= curve(px) && py <= base,
        |_, py| {
            let k = ((base - py) / span).clamp(0.0, 1.0);
            (color, style.at_base + (style.at_curve - style.at_base) * k)
        },
    );

    if !style.outline {
        return;
    }

    // The curve itself, drawn as a vertical band around it: at these sizes a
    // true distance-to-segment stroke and a vertical thickness are within a
    // pixel of each other, and this one costs a single evaluation per sample.
    c.fill((x, y, w, h), color, 0.95, |px, py| {
        px >= x && px <= x + w && (py - curve(px)).abs() <= STROKE * 0.5
    });

    // A dot on the newest reading: the eye lands on it first and reads the
    // series backwards from there.
    let hx = x + w - STROKE;
    let hy = curve(x + w);
    let r = STROKE * 1.15;
    c.fill((hx - r, hy - r, 2.0 * r, 2.0 * r), color, 1.0, |px, py| {
        (px - hx).powi(2) + (py - hy).powi(2) <= r * r
    });
}

/// The series' value at `t` in `0.0..=1.0`, Catmull–Rom interpolated between
/// samples so the curve is smooth without overshooting into a shape the data
/// never took (the tangents are clamped by the neighbouring samples).
pub fn value_at(samples: &[f32], t: f32) -> f32 {
    let n = samples.len();
    if n == 1 {
        return samples[0];
    }
    let pos = t.clamp(0.0, 1.0) * (n - 1) as f32;
    let i = (pos.floor() as usize).min(n - 2);
    let f = pos - i as f32;
    let at = |k: isize| samples[k.clamp(0, n as isize - 1) as usize];
    let (p0, p1, p2, p3) = (
        at(i as isize - 1),
        at(i as isize),
        at(i as isize + 1),
        at(i as isize + 2),
    );
    let v = 0.5
        * ((2.0 * p1)
            + (-p0 + p2) * f
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * f * f
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * f * f * f);
    // Never leave the interval its two bracketing samples define: an overshoot
    // here would draw a spike that was never measured.
    v.clamp(p1.min(p2), p1.max(p2))
}

#[cfg(test)]
mod tests {
    use super::{draw, value_at};
    use crate::plot::Canvas;

    #[test]
    fn the_curve_passes_through_every_sample() {
        let s = [0.0, 1.0, 0.25, 0.8];
        for (i, want) in s.iter().enumerate() {
            let t = i as f32 / (s.len() - 1) as f32;
            assert!((value_at(&s, t) - want).abs() < 1e-4, "sample {i}");
        }
    }

    #[test]
    fn interpolation_never_overshoots_the_data() {
        // A step the naive spline would ring on: the curve must stay inside
        // the two samples it is between, or the chart shows a spike the
        // machine never had.
        let s = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0];
        for k in 0..=200 {
            let t = k as f32 / 200.0;
            let v = value_at(&s, t);
            assert!((-1e-5..=1.0 + 1e-5).contains(&v), "t={t} v={v}");
        }
    }

    #[test]
    fn a_flat_series_fills_its_share_of_the_box() {
        // Half-height series → the fill covers about half the chart, and the
        // *shape* carries the reading (the glyph sparkline it replaced had
        // eight levels total, so 0.30 and 0.37 drew the same row).
        let mut c = Canvas::new(20, 2, 2.0);
        let (w, h) = c.size();
        draw(&mut c, (0.0, 0.0, w, h), &[0.5; 8], (0, 200, 255));
        let ink: f32 = c
            .paint()
            .iter()
            .map(|p| p.w * p.h * c.row_units() * p.alpha)
            .sum();
        let box_area = w * h;
        // Fill fades from 0.38 to 0.04 over the covered half, plus the stroke.
        assert!(
            (0.06..0.20).contains(&(ink / box_area)),
            "covered fraction {}",
            ink / box_area
        );
    }

    #[test]
    fn a_higher_series_paints_more_than_a_lower_one() {
        let ink = |v: f32| {
            let mut c = Canvas::new(20, 2, 2.0);
            let (w, h) = c.size();
            draw(&mut c, (0.0, 0.0, w, h), &[v; 8], (0, 200, 255));
            c.paint().iter().map(|p| p.w * p.h * p.alpha).sum::<f32>()
        };
        let (low, mid, high) = (ink(0.1), ink(0.5), ink(0.95));
        assert!(low < mid && mid < high, "{low} < {mid} < {high}");
    }

    #[test]
    fn an_empty_series_draws_nothing() {
        let mut c = Canvas::new(10, 2, 2.0);
        draw(&mut c, (0.0, 0.0, 10.0, 4.0), &[], (0, 0, 0));
        assert!(c.paint().is_empty());
    }

    #[test]
    fn the_newest_sample_sits_at_the_right_edge() {
        // Zero everywhere but the last reading: the ink must be on the right.
        let mut c = Canvas::new(20, 2, 2.0);
        let (w, h) = c.size();
        let mut s = [0.0f32; 10];
        s[9] = 1.0;
        draw(&mut c, (0.0, 0.0, w, h), &s, (0, 200, 255));
        let top: Vec<_> = c
            .paint()
            .into_iter()
            .filter(|p| p.y < h / c.row_units() * 0.25)
            .collect();
        assert!(!top.is_empty(), "the peak reached the top of the box");
        assert!(
            top.iter().all(|p| p.x > 15.0),
            "the peak is at the right edge: {:?}",
            top.iter().map(|p| p.x).collect::<Vec<_>>()
        );
    }
}
