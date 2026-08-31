//! The two rolling-window meters under the crew pane's summary: the track,
//! the shading, and the art that draws them.
//!
//! Split from [`crate::chatsummary`] for the line cap.
use crate::chat::ChatPane;
use crate::chatsummary::*;
use crew_render::{CellView, Paint};

/// How far an unfilled meter is pulled back toward the page. Far enough that
/// the fill level reads without measuring, not so far that the track
/// disappears and the meter loses its length.
pub(crate) const TROUGH_FADE: f32 = 0.6;

/// The colour a drawn meter's fill has at `t` along its length: the theme's
/// own gradient, `pole_a` at the left edge to `pole_b` at the right, so a
/// filling meter walks the ramp the rest of the app is lit by. Falls back to
/// the muted ink on a theme without a `ModernStyle`.
pub(crate) fn meter_shade(t: f32) -> (u8, u8, u8) {
    crate::modernring::pole_mix(t).unwrap_or_else(|| crew_theme::theme().text_muted)
}

/// The track colour under it — the same ramp, pulled toward the page.
pub(crate) fn meter_track() -> (u8, u8, u8) {
    crate::anim::lerp_rgb(meter_shade(0.5), crew_theme::theme().page_bg, TROUGH_FADE)
}

/// The most rows the footer ever claims (identity/spend, windows/bars,
/// routing mode).
pub(crate) const MAX_BLOCK: u16 = 3;

/// The footer's cells *and* the meters drawn under them. `aspect` is the
/// frame's `cell_h / cell_w`, so the capsules keep their proportions at any
/// font size.
///
/// One pass, not two: the readouts animating the numbers are ticked here, and
/// building the footer twice a frame would tick them twice.
pub(crate) fn summary_art(
    pane: &ChatPane,
    cols: u16,
    top: u16,
    height: u16,
    aspect: f32,
) -> (Vec<CellView>, Vec<Paint>) {
    if height == 0 {
        return (Vec::new(), Vec::new());
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut meters: Vec<f32> = Vec::new();
    let lines = footer_lines_with(&footer_ctx(pane, now_ms), cols as usize, &mut meters);
    let bg = crew_theme::theme().page_bg;
    let mut cells = Vec::new();
    for (i, line) in lines.into_iter().take(height as usize).enumerate() {
        let row = top + i as u16;
        crate::chatwidth::place_row(1, cols, line, |x, c, fg| {
            cells.push(CellView {
                col: x,
                row,
                c,
                fg,
                bg,
                bold: false,
                italic: false,
                ..Default::default()
            });
        });
    }
    let paint = draw_meters(&mut cells, &meters, aspect);
    (cells, paint)
}

/// Replace each reserved glyph run with a drawn capsule.
///
/// The run is found in the *placed* cells, so the meter lands exactly where
/// the line budget put it — nothing here has to know how the line was laid
/// out. The glyphs are blanked as they are found: a `▓` showing through a
/// drawn meter is the same bug as two charts for one number.
pub(crate) fn draw_meters(cells: &mut [CellView], meters: &[f32], aspect: f32) -> Vec<Paint> {
    let is_meter = |c: char| c == FILLED || c == EMPTY;
    let mut runs: Vec<(u16, u16, u16)> = Vec::new(); // row, col, width
    let mut idx: Vec<usize> = (0..cells.len()).collect();
    idx.sort_by_key(|&i| (cells[i].row, cells[i].col));
    let mut k = 0;
    while k < idx.len() {
        if !is_meter(cells[idx[k]].c) {
            k += 1;
            continue;
        }
        let (row, col) = (cells[idx[k]].row, cells[idx[k]].col);
        let start = k;
        while k < idx.len()
            && is_meter(cells[idx[k]].c)
            && cells[idx[k]].row == row
            && cells[idx[k]].col == col + (k - start) as u16
        {
            cells[idx[k]].c = ' ';
            k += 1;
        }
        runs.push((row, col, (k - start) as u16));
    }

    let track = meter_track();
    let mut out = Vec::new();
    for ((row, col, w), &frac) in runs.iter().zip(meters) {
        // One row, a handful of columns: a fine grid costs nothing here, and
        // the pill's end caps are a fraction of a column across — on the
        // default grid they had under two pixels to round with and came out
        // square.
        let mut c = crate::plot::Canvas::new(*w, 1, aspect);
        crate::plot::meter::capsule(
            &mut c,
            0.0,
            0.0,
            f32::from(*w),
            aspect,
            frac,
            meter_shade,
            track,
        );
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(f32::from(*col), f32::from(*row))),
        );
    }
    out
}
