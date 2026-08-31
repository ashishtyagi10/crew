//! A timeline: one bar per row, all on one time axis.
//!
//! A swarm's task list says which tasks ran and how they ended. What it cannot
//! say is *when* — and "when" is the whole question about a swarm, because a
//! scheduler that runs six tasks one after another and one that runs them at
//! once produce identical lists. Six bars stacked in a column answer it before
//! you have read a word.
use crate::plot::Canvas;

/// One bar: a span in milliseconds and the colour it is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start_ms: u64,
    pub end_ms: u64,
    pub color: (u8, u8, u8),
}

/// How tall a bar is inside its row.
const BAR: f32 = 0.42;

/// Draw `spans` — one per row, in order — across `(x, y, w)` with `row_h`
/// units per row, on the axis `t0..t1`.
///
/// A zero-length span (a task that started and finished inside one frame)
/// still draws a mark: at the resolutions a pane has, most fast tasks are
/// zero-length, and a scheduler firing thirty of them is exactly what you want
/// to see.
pub fn draw(
    c: &mut Canvas,
    rect: (f32, f32, f32),
    row_h: f32,
    spans: &[Option<Span>],
    t0: u64,
    t1: u64,
    track: (u8, u8, u8),
) {
    let (x, y, w) = rect;
    if w <= 0.0 || spans.is_empty() {
        return;
    }
    let span_ms = t1.saturating_sub(t0).max(1) as f32;
    let at = |ms: u64| x + (ms.saturating_sub(t0) as f32 / span_ms).clamp(0.0, 1.0) * w;
    let h = (row_h * BAR).max(0.12);
    for (i, span) in spans.iter().enumerate() {
        let top = y + i as f32 * row_h + (row_h - h) * 0.5;
        // The row's own track: an unrun task still has a lane, so the rows
        // line up with the list beside them.
        c.rect(x, top + h * 0.5 - 0.03, w, 0.06, track, 0.55);
        let Some(s) = span else { continue };
        let (bx, bw) = (at(s.start_ms), (at(s.end_ms) - at(s.start_ms)).max(h * 0.6));
        c.rect(bx, top, bw.min(x + w - bx).max(0.05), h, s.color, 0.95);
    }
}

/// A vertical rule across every row — the axis mark for "now", or a tick.
pub fn rule(c: &mut Canvas, x: f32, y: f32, h: f32, color: (u8, u8, u8), alpha: f32) {
    c.rect(x - 0.03, y, 0.06, h, color, alpha);
}

#[cfg(test)]
#[path = "gantt_tests.rs"]
mod tests;
