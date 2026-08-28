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
use crate::plot::Canvas;

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
    let mut c = Canvas::new(cols, rows, aspect);
    thumb(&mut c, cols, rows, b, aspect);
    progress(&mut c, cols, rows, b, aspect, now);
    c.paint()
}

/// The scroll thumb, down the right border — a rounded bar at a fractional
/// position, so a long buffer scrolls smoothly instead of in cell steps.
fn thumb(c: &mut Canvas, cols: u16, rows: u16, b: &Bar, aspect: f32) {
    if (b.scroll == 0 && !b.doc) || rows < MIN_ROWS || cols < 2 || b.total == 0 {
        return;
    }
    let visible = usize::from(rows - 2);
    if b.total <= visible {
        return;
    }
    let first = b.total.saturating_sub(visible).saturating_sub(b.scroll);
    // Where the window sits in the buffer, and how much of it it covers.
    let span = (visible as f32 / b.total as f32).clamp(0.02, 1.0);
    let at = (first as f32 / b.total.saturating_sub(visible).max(1) as f32).clamp(0.0, 1.0);
    let track_top = aspect; // below the top border row
    let track_h = (rows - 2) as f32 * aspect;
    let h = (track_h * span).max(aspect * 0.6);
    let y = track_top + at * (track_h - h);
    let x = f32::from(cols - 1) + (1.0 - WEIGHT) * 0.5;
    let fg =
        crate::panescroll::position_fg(crate::panescroll::position(b.total, visible, b.scroll));
    rounded_v(c, x, y, WEIGHT, h, fg, 0.95);
}

/// The program's own progress (OSC 9;4) along the bottom border.
fn progress(c: &mut Canvas, cols: u16, rows: u16, b: &Bar, aspect: f32, now: u64) {
    let Some(p) = b.progress else { return };
    let t = crew_theme::theme();
    let fg = match p.alarm {
        true => t.bell,
        false => t.activity,
    };
    let inner = f32::from(cols - 2);
    let y = f32::from(rows - 1) * aspect + (aspect - WEIGHT) * 0.5;
    match p.percent {
        Some(pct) => {
            let w = inner * f32::from(pct.min(100)) / 100.0;
            if w > 0.0 {
                c.rect(1.0, y, w, WEIGHT, fg, 0.95);
            }
        }
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
            c.fill_shaded(
                (x0, y, w, WEIGHT),
                |px, py| px >= x0 && px <= x0 + w && py >= y && py <= y + WEIGHT,
                |px, _| {
                    let t = ((px - x0) / w).clamp(0.0, 1.0);
                    let lead = if forward { t } else { 1.0 - t };
                    (fg, lead.powf(1.6) * 0.95)
                },
            );
        }
    }
}

/// A vertical rounded bar (the thumb's shape).
fn rounded_v(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, color: (u8, u8, u8), alpha: f32) {
    let r = (w * 0.5).min(h * 0.5);
    c.fill((x, y, w, h), color, alpha, move |px, py| {
        if px < x || px > x + w || py < y || py > y + h {
            return false;
        }
        let dy = if py < y + r {
            y + r - py
        } else if py > y + h - r {
            py - (y + h - r)
        } else {
            0.0
        };
        let dx = (px - (x + w * 0.5)).abs();
        dx * dx + dy * dy <= r * r || dy == 0.0
    });
}
