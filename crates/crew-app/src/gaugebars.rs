//! The nav's SYSTEM readings as drawn capsules, when it is too narrow for
//! the dials.
//!
//! Below [`crate::sysdials::fits`] the three readings fell back to `█░` text
//! bars, and three rows of full blocks stacked flush drew one white slab —
//! CPU, MEM and DISK fused into an `L`. Every other meter in the nav (SERVING,
//! the rail's C/M/D) is a drawn capsule; these are the same capsule, in the
//! load tier's colour, laid where the glyph bar was (`gauges::gauge_cells`,
//! whose bar the cell pass now leaves blank).
use crew_render::Paint;

use crate::gauges::{fill_color, track_color};
use crate::plot::Canvas;
use crate::stats::Stats;

/// Columns before a bar: the section indent, the 4-wide label, the tier mark.
const BAR_COL: u16 = 3 + 4 + 1;
/// Columns after it: the `NNN%` reading and the nav's right air.
const TAIL: u16 = 4 + 1;

/// The capsules for a nav `cols` wide whose SYSTEM gauges start on `row0`.
/// Nothing for the dashboard (`rows` names which dial block asked — its
/// narrow form has no bars to stand in for) or when the dials fit.
pub(crate) fn paint(rows: u16, stats: Stats, cols: u16, row0: u16, aspect: f32) -> Vec<Paint> {
    let w = cols.saturating_sub(BAR_COL + TAIL);
    if rows != crate::sysdials::NAV.rows || crate::sysdials::fits(cols) || w < 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, frac) in [stats.cpu, stats.mem, stats.disk].into_iter().enumerate() {
        let mut c = Canvas::new(w, 1, aspect);
        let fg = fill_color(frac);
        crate::plot::meter::capsule(
            &mut c,
            0.0,
            0.0,
            f32::from(w),
            aspect,
            frac,
            |_| fg,
            track_color(),
        );
        let row = f32::from(row0 + i as u16);
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(f32::from(BAR_COL), row)),
        );
    }
    out
}

#[cfg(test)]
#[path = "gaugebars_tests.rs"]
mod tests;
