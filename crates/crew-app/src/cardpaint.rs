//! The pane card's own indicators, drawn instead of spelled.
//!
//! A card carries two continuous readings on its frame: how far down the
//! buffer you are (the right-border thumb) and how far along the program in
//! it says it is (the bottom-border progress bar). Both were runs of
//! box-drawing glyphs, so both moved a whole cell at a time — a thumb in a
//! 40-row pane could only stop at 38 places in a 200,000-line scrollback, and
//! a build at 3% and one at 5% drew the same bar.
//!
//! Drawn, they are continuous: the thumb slides, the bar lands where the
//! number says, and the indeterminate sweep is a comet with a fading tail
//! rather than a block that jumps.
use crew_render::Paint;

use crate::panecard::Bar;
use crate::plot::{sdf, Canvas};

/// Thickness of a border indicator, in cell widths.
const WEIGHT: f32 = 0.34;
/// One full bounce of the indeterminate sweep, in ms — the same clock the
/// glyph sweep used, so nothing about the motion changed but its smoothness.
const SWEEP_MS: u64 = 1400;
/// Rows a card needs before its gutter is worth drawing.
const MIN_ROWS: u16 = 5;

/// Everything this card draws on its frame, in the card's own cell
/// coordinates (`0,0` is the top-left border cell).
pub fn card_paint(cols: u16, rows: u16, b: &Bar, aspect: f32, now: u64) -> Vec<Paint> {
    if cols < 4 || rows < 3 {
        return Vec::new();
    }
    // A strip each, not one buffer the size of the card: the old shared
    // canvas allocated and zeroed `cols * rows * SUB^2` pixels every frame
    // for every pane — megabytes on a wide card — to put ink on two edges of
    // it.
    let mut out = thumb(cols, rows, b, aspect);
    out.extend(progress(cols, rows, b, aspect, now));
    out
}

/// The scroll thumb, down the right border — a rounded bar at a fractional
/// position, so a long buffer scrolls smoothly instead of in cell steps.
fn thumb(cols: u16, rows: u16, b: &Bar, aspect: f32) -> Vec<Paint> {
    if (b.scroll == 0 && !b.doc) || rows < MIN_ROWS || cols < 2 || b.total == 0 {
        return Vec::new();
    }
    let visible = usize::from(rows - 2);
    if b.total <= visible {
        return Vec::new();
    }
    let first = b.total.saturating_sub(visible).saturating_sub(b.scroll);
    // Where the window sits in the buffer, and how much of it it covers.
    let span = (visible as f32 / b.total as f32).clamp(0.02, 1.0);
    let at = (first as f32 / b.total.saturating_sub(visible).max(1) as f32).clamp(0.0, 1.0);
    let track_top = aspect; // below the top border row
    let track_h = (rows - 2) as f32 * aspect;
    let h = (track_h * span).max(aspect * 0.6);
    let y = track_top + at * (track_h - h);
    // The strip is the border column itself; the bar is placed inside it.
    let x = (1.0 - WEIGHT) * 0.5;
    let fg =
        crate::panescroll::position_fg(crate::panescroll::position(b.total, visible, b.scroll));
    let mut c = Canvas::new(1, rows, aspect);
    c.fill_sdf((x, y, WEIGHT, h), fg, 0.95, move |px, py| {
        sdf::round_box((px, py), x, y, WEIGHT, h, WEIGHT * 0.5)
    });
    c.paint()
        .into_iter()
        .map(|p| p.shifted(f32::from(cols - 1), 0.0))
        .collect()
}

/// The program's own progress (OSC 9;4) along the bottom border.
fn progress(cols: u16, rows: u16, b: &Bar, aspect: f32, now: u64) -> Vec<Paint> {
    let Some(p) = b.progress else {
        return Vec::new();
    };
    let t = crew_theme::theme();
    let fg = match p.alarm {
        true => t.bell,
        false => t.activity,
    };
    let inner = f32::from(cols - 2);
    // One row tall, at the card's last row: `y` is inside the strip.
    let y = (aspect - WEIGHT) * 0.5;
    let mut c = Canvas::new(cols, 1, aspect);
    match p.percent {
        // A FULL bar is not a reading, it is a line. A program that reports
        // 100 and then keeps working — which is most of them, since almost
        // nobody clears OSC 9;4 before exiting — left a saturated stroke
        // pinned under the pane for the rest of the session, with nothing on
        // screen saying what it was. Every progress bar outside a terminal
        // disappears when it fills; this one does too.
        Some(pct) if pct < 100 => {
            let w = inner * f32::from(pct) / 100.0;
            if w > 0.0 {
                // A pill, like every other bar in the app: at a third of a
                // column the cap is a pixel and a half, which is a rounded
                // end the sampled path could not draw at all.
                c.fill_sdf((1.0, y, w, WEIGHT), fg, 0.95, move |px, py| {
                    sdf::round_box((px, py), 1.0, y, w, WEIGHT, WEIGHT * 0.5)
                });
            }
        }
        Some(_) => {}
        None => {
            // A comet: a block of the same width the glyph sweep used, but
            // shaded so its LEADING edge is brightest and it fades away
            // behind. A block of constant brightness cannot say which way it
            // is going, and half of a bounce is spent going the other way.
            let w = (inner / 5.0).max(1.0);
            let travel = (inner - w).max(0.0);
            let phase = crate::anim::tri(now, SWEEP_MS);
            let x0 = 1.0 + phase * travel;
            // Which way it is heading, from the wave a moment later.
            let forward = crate::anim::tri(now + SWEEP_MS / 64, SWEEP_MS) >= phase;
            c.fill_sdf_shaded(
                (x0, y, w, WEIGHT),
                move |px, py| sdf::round_box((px, py), x0, y, w, WEIGHT, WEIGHT * 0.5),
                move |px, _| {
                    let t = ((px - x0) / w).clamp(0.0, 1.0);
                    let lead = if forward { t } else { 1.0 - t };
                    (fg, lead.powf(1.6) * 0.95)
                },
            );
        }
    }
    c.paint()
        .into_iter()
        .map(|p| p.shifted(0.0, f32::from(rows - 1)))
        .collect()
}
