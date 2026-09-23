//! The cost-per-day chart `/dash` and `/usage` both draw: a bar per day
//! (`plot::bars`), each LABELLED with what it cost, and each day named on
//! the axis under it.
//!
//! The bars came with one number — `peak $0.90` in the legend — so every
//! other day was a height you had to estimate against it, and the axis named
//! only its two ends. Seven bars have the room to say their own values, and a
//! direct label is read where the eye already is, not decoded from a scale.
//! The bars stand one row short of the chart's top so the tallest one has a
//! row above it for its number; the day names are the heatmap's own
//! (`6d … now`), so the two charts on the page count days the same way.
use crew_render::{CellView, Paint};

use crate::plot::Canvas;
use crate::usagelayout::{day_labels, money};

/// The bars for `daily` (micro-USD per day, oldest first) on a canvas
/// `w_cells × rows`, unshifted. With two rows or more the top row is left
/// clear for the value labels.
pub(crate) fn paint(daily: &[u64], w_cells: u16, rows: u16, aspect: f32) -> Vec<Paint> {
    let peak = daily.iter().copied().max().unwrap_or(0).max(1);
    let values: Vec<f32> = daily.iter().map(|&v| v as f32 / peak as f32).collect();
    let mut c = Canvas::new(w_cells, rows, aspect);
    let (w, h) = c.size();
    let head = if rows >= 2 { c.row_units() } else { 0.0 };
    let color = crew_theme::theme().ansi[11];
    crate::plot::bars::draw(&mut c, (0.0, head, w, h - head), &values, color);
    c.paint()
}

/// Value labels over the bars and day names on `axis`, for bars drawn by
/// [`paint`] at column `left`, `w_cells` wide, from row `top` over `rows`.
/// Where a slot is too narrow for its words, the axis falls back to naming
/// the ends of the week and the values are left to the legend's peak.
#[allow(clippy::too_many_arguments)]
pub(crate) fn labels(
    out: &mut Vec<CellView>,
    daily: &[u64],
    left: u16,
    w_cells: u16,
    top: u16,
    rows: u16,
    axis: u16,
    cols: u16,
) {
    let t = crew_theme::theme();
    let n = daily.len().max(1);
    let slot = f32::from(w_cells) / n as f32;
    let centre = |i: usize, s: &str| {
        let mid = f32::from(left) + slot * (i as f32 + 0.5);
        (mid - s.chars().count() as f32 / 2.0).round().max(0.0) as u16
    };
    let names = day_labels();
    let fits = |s: &str| s.chars().count() as f32 + 1.0 <= slot;
    if names.iter().all(|s| fits(s)) {
        for (i, s) in names.iter().enumerate().take(daily.len()) {
            crate::navtext::put_at(out, s, centre(i, s), axis, cols, t.text_muted);
        }
    } else {
        crate::usageaxis::week_ends(out, axis, cols, cols.saturating_sub(left + w_cells));
    }
    let peak = daily.iter().copied().max().unwrap_or(0).max(1);
    if rows < 2 {
        return;
    }
    for (i, &v) in daily.iter().enumerate() {
        let s = money(v);
        if v == 0 || !fits(&s) {
            continue;
        }
        // The bar stands on rows `top+1 .. top+rows`; its label sits on the
        // row above its top cell.
        let bar = (v as f32 / peak as f32 * f32::from(rows - 1)).ceil() as u16;
        let row = (top + rows).saturating_sub(bar + 1).max(top);
        crate::navtext::put_at(out, &s, centre(i, &s), row, cols, t.ink);
    }
}

#[cfg(test)]
#[path = "costbars_tests.rs"]
mod tests;
