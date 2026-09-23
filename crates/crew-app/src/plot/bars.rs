//! Bar chart: one bar per value, for a series whose points are separate
//! totals rather than readings of something continuous.
//!
//! Cost per day was drawn with [`super::area`], a smooth curve through seven
//! daily sums. A curve says the quantity flows between its samples; a day's
//! spend does not — Tuesday's $0.42 and Thursday's $0.04 have no Wednesday
//! afternoon between them. The spline invented slopes the data never had
//! (a zero day became a gentle valley floor, a peak day a rounded hill whose
//! top sat between two dates), and seven points on a curve do not read as
//! seven days. One bar per day is the form every dashboard uses for daily
//! totals: you count the bars, compare their heights, and see a zero day as
//! the stub it is.
use crate::plot::Canvas;

/// Share of each slot left empty between neighbours, so bars read as
/// separate days rather than a skyline.
const GAP: f32 = 0.28;
/// The body's alpha: solid enough to compare heights, lighter than the cap
/// so the value line is where the eye lands (the area chart's curve-over-
/// fill, in bar form).
const BODY: f32 = 0.55;
/// A zero value's stub: present (the day exists, it cost nothing), faint.
const STUB: f32 = 0.35;

/// Draw `values` (each 0..=1 of the box height) as bars across `rect`
/// (`x, y, w, h` in canvas units), left to right, in `color`.
pub fn draw(c: &mut Canvas, rect: (f32, f32, f32, f32), values: &[f32], color: (u8, u8, u8)) {
    let (x, y, w, h) = rect;
    if values.is_empty() || w <= 0.0 || h <= 0.0 {
        return;
    }
    let px = c.px();
    let slot = w / values.len() as f32;
    let gap = (slot * GAP).max(px);
    let bw = (slot - gap).max(px);
    for (i, &v) in values.iter().enumerate() {
        let bx = x + i as f32 * slot + gap / 2.0;
        let v = v.clamp(0.0, 1.0);
        let bh = v * h;
        if bh < px {
            c.rect(bx, y + h - px, bw, px, color, STUB);
            continue;
        }
        let top = y + h - bh;
        c.rect(bx, top + px, bw, bh - px, color, BODY);
        c.rect(bx, top, bw, px, color, 1.0);
    }
}

#[cfg(test)]
#[path = "bars_tests.rs"]
mod tests;
