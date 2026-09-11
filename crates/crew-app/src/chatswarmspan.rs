//! Each task's span on the run's clock, as a thin bar of cells: the timeline
//! half of a task row in the chat's swarm block (`chatswarmrows`).
//!
//! The `/swarm` pane draws its Gantt with the paint layer (`swarm/timeline`
//! and `plot::gantt`); the chat block is a few rows inside a transcript and
//! gets cells instead, one `━` run per task on a shared `─` axis coloured by
//! state, so it costs the frame nothing and reads at a glance. The clock is
//! the frame clock the model stamps on (`SwarmTask::started_ms`), and
//! `now_ms` is threaded in so a frame and a test see the same axis.
use crate::chatswarm::SwarmTask;
use crate::shimmer::Color;

/// The bar's cell, and the axis' where the task has no span.
const SPAN: char = '\u{2501}'; // ━
const AXIS: char = '\u{2500}'; // ─

/// Columns the bar takes on a `cols`-wide block, or 0 when the block is too
/// narrow to hold both the bar and a title worth reading — below that the
/// bar goes first, never the title.
pub(crate) fn bar_cols(cols: u16) -> u16 {
    if cols < 56 {
        0
    } else {
        (cols / 5).clamp(10, 24)
    }
}

/// The axis `(t0, t1)` at `now_ms`: from the first start to now while any
/// task runs, else to the last end. `None` until a task has started. Never
/// zero-width, so a just-started run does not put every bar on one column.
pub(crate) fn axis(tasks: &[SwarmTask], now_ms: u64) -> Option<(u64, u64)> {
    let t0 = tasks.iter().filter_map(|t| t.started_ms).min()?;
    let live = tasks
        .iter()
        .any(|t| t.started_ms.is_some() && t.ended_ms.is_none());
    let t1 = if live {
        now_ms
    } else {
        tasks.iter().filter_map(|t| t.ended_ms).max().unwrap_or(t0)
    };
    Some((t0, t1.max(t0 + 1)))
}

/// The bar for `t` over `w` columns: the axis in `muted`, its span in
/// `color` (a running span reaches `now_ms`). A task that has not started
/// is all axis.
pub(crate) fn bar(
    t: &SwarmTask,
    (t0, t1): (u64, u64),
    w: u16,
    now_ms: u64,
    color: Color,
    muted: Color,
) -> Vec<(char, Color)> {
    let w = usize::from(w);
    let mut cells = vec![(AXIS, muted); w];
    let Some(start) = t.started_ms else {
        return cells;
    };
    let end = t.ended_ms.unwrap_or(now_ms).max(start);
    let span = (t1 - t0) as f64;
    let at = |ms: u64| ((ms.clamp(t0, t1) - t0) as f64 / span * w as f64) as usize;
    let x0 = at(start).min(w.saturating_sub(1));
    let x1 = at(end).max(x0 + 1).min(w);
    for c in cells.iter_mut().take(x1).skip(x0) {
        *c = (SPAN, color);
    }
    cells
}

/// `12s` / `4m 12s` — the block's elapsed wording (`chatswarmview` uses the
/// same shape on the status line).
pub(crate) fn fmt_ms(ms: u64) -> String {
    crate::chatswarmview::fmt_elapsed_short(ms / 1000)
}

#[cfg(test)]
#[path = "chatswarmspan_tests.rs"]
mod tests;
