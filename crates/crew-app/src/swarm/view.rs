//! Fleet → CellViews renderer: a legible task list over live fleet telemetry.
//! Row 0 is a HUD of fleet totals; each row below is one task — state glyph,
//! title, and (while running or after failing) the agent's last output line —
//! so a swarm pane shows *what* is happening, not just how much.
//!
//! Rendered through a ratatui `Buffer` (not hand-placed cells) so column
//! arithmetic is width-aware: emoji or CJK in a title/output line occupy two
//! cells and the text after them still lands on the right column.
use std::collections::HashMap;

use crew_hive::{Fleet, TaskGraph, TaskState};
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

fn style(fg: (u8, u8, u8), bold: bool) -> Style {
    let s = Style::new().fg(rgb(fg));
    if bold {
        s.add_modifier(Modifier::BOLD)
    } else {
        s
    }
}

/// Columns the timeline claims on the right of a swarm pane, and the width
/// below which it is not drawn at all (a pane too narrow for both loses the
/// chart, never the task names).
pub fn timeline_cols(cols: u16) -> u16 {
    match cols {
        // Below this the task titles would be cut to a few characters, and a
        // chart of tasks you cannot name is worth less than the names.
        0..=45 => 0,
        _ => (cols / 3).clamp(14, 40),
    }
}

/// The colour a task's bar takes on the timeline — the same colour its glyph
/// wears in the list, so the two halves of the pane agree.
pub fn state_color(state: TaskState) -> (u8, u8, u8) {
    state_style(state).1
}

/// Glyph, colour, and bold flag for a task state.
fn state_style(state: TaskState) -> (char, (u8, u8, u8), bool) {
    let t = crew_theme::theme();
    match state {
        TaskState::Pending | TaskState::Ready => ('\u{25cb}', t.text_muted, false), // ○
        TaskState::Running => ('\u{25cf}', crate::palette::accent(), true),         // ●
        TaskState::Done => ('\u{2713}', t.ansi[2], false),                          // ✓
        TaskState::Failed => ('\u{2717}', t.ansi[9], true),                         // ✗
        TaskState::Cancelled => ('\u{2013}', t.text_muted, false),                  // –
    }
}

/// Map a `Fleet` to a `Vec<CellView>` for the given terminal grid.
///
/// Row 0 is a HUD showing live/done/failed/cost totals. Rows 1‥rows-1 list the
/// graph's tasks in order, one per row, with a trailing `… +N more` overflow
/// row when the pane is too short for them all.
///
/// Returns an empty vec when `cols == 0 || rows == 0`.
pub fn swarm_cells(graph: &TaskGraph, fleet: &Fleet, cols: u16, rows: u16) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return vec![];
    }
    let t = crew_theme::theme();
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));

    // HUD row: live/done/failed + cost, in the widest form the width holds.
    let totals = fleet.totals();
    let hud = super::rows::hud_text(
        totals.live,
        totals.done,
        totals.failed,
        totals.micros_usd,
        cols,
    );
    buf.set_line(0, 0, &Line::styled(hud, style(t.ink, false)), cols);

    // Task rows below the HUD. A task with no spawned agent yet is Pending.
    let by_task: HashMap<_, _> = fleet.agents().map(|a| (a.task, a)).collect();
    let tasks = graph.tasks();
    let shown = super::rows::shown(tasks.len(), rows);
    for (i, spec) in tasks.iter().take(shown).enumerate() {
        let agent = by_task.get(&spec.id);
        let state = agent.map_or(TaskState::Pending, |a| a.state);
        let (glyph, color, bold) = state_style(state);
        // The live tail: what the agent last printed (or the failure reason).
        let tail = agent
            .filter(|_| matches!(state, TaskState::Running | TaskState::Failed))
            .map(|a| a.last_line.as_str())
            .unwrap_or_default();
        let (title, tail) = super::rows::task_row(&spec.title, tail, cols);
        let mut spans = vec![
            Span::styled(format!(" {glyph} "), style(color, bold)),
            Span::styled(title, style(color, bold)),
        ];
        if !tail.is_empty() {
            spans.push(Span::styled(tail, style(t.text_muted, false)));
        }
        buf.set_line(0, (i + 1) as u16, &Line::from(spans), cols);
    }
    if tasks.len() > shown && rows > 1 {
        let note = format!(" \u{2026} +{} more", tasks.len() - shown);
        buf.set_line(
            0,
            (shown + 1) as u16,
            &Line::styled(note, style(t.text_muted, false)),
            cols,
        );
    }
    crate::tui::to_cells(&buf)
}

/// The timeline's own text: the axis' span on the HUD row, over the bars.
/// `axis` is `(t0, t1)` in wall milliseconds; nothing is drawn without one.
pub fn timeline_cells(cols: u16, rows: u16, axis: Option<(u64, u64)>) -> Vec<CellView> {
    let w = timeline_cols(cols);
    let Some((t0, t1)) = axis else {
        return vec![];
    };
    if w == 0 || rows == 0 {
        return vec![];
    }
    let t = crew_theme::theme();
    let x0 = cols - w;
    // "0s" at the left of the axis and the elapsed span at its right: two
    // labels are all a chart this narrow can carry, and they are the two that
    // say what the bars are measured against.
    let elapsed = format!("{:.0}s", (t1.saturating_sub(t0)) as f64 / 1000.0);
    let mut out = Vec::new();
    let put = |out: &mut Vec<CellView>, s: &str, col: u16| {
        for (i, c) in s.chars().enumerate() {
            let col = col + i as u16;
            if col >= cols {
                break;
            }
            out.push(CellView {
                col,
                row: 0,
                c,
                fg: t.text_muted,
                bg: t.page_bg,
                ..Default::default()
            });
        }
    };
    put(&mut out, "0s", x0 + 1);
    put(
        &mut out,
        &elapsed,
        cols.saturating_sub(elapsed.chars().count() as u16 + 1),
    );
    out
}

/// The bars themselves. One row per task, lined up with the list beside them,
/// with a rule at the newest instant so a live swarm shows its own edge.
pub fn timeline_paint(
    spans: &[Option<crate::plot::gantt::Span>],
    cols: u16,
    rows: u16,
    aspect: f32,
    axis: (u64, u64),
    now: u64,
) -> Vec<crew_render::Paint> {
    let w = timeline_cols(cols);
    if w == 0 || rows < 2 || spans.is_empty() {
        return Vec::new();
    }
    let t = crew_theme::theme();
    // Rows 1.. are the task rows; row 0 is the HUD.
    let lanes = (rows - 1).min(spans.len() as u16);
    let mut c = crate::plot::Canvas::new(w, lanes, aspect);
    let (cw, ch) = c.size();
    let inset = 1.0; // a column of air either side, so bars clear the list
    crate::plot::gantt::draw(
        &mut c,
        (inset, 0.0, cw - 2.0 * inset),
        ch / f32::from(lanes),
        &spans[..lanes as usize],
        axis.0,
        axis.1,
        t.border_normal,
    );
    // Where "now" is on the axis — at the right edge while anything runs, and
    // wherever the last task ended once nothing does.
    let span = axis.1.saturating_sub(axis.0).max(1) as f32;
    let x = inset + (now.saturating_sub(axis.0) as f32 / span).clamp(0.0, 1.0) * (cw - 2.0 * inset);
    crate::plot::gantt::rule(&mut c, x, 0.0, ch, crate::palette::accent(), 0.5);
    c.paint()
        .into_iter()
        .map(|p| p.shifted(f32::from(cols - w), 1.0))
        .collect()
}

/// An amber notice on the last row when the budget governor stopped a swarm, so
/// a cancelled run doesn't just look "done".
pub fn cancelled_notice(cols: u16, rows: u16) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return vec![];
    }
    let t = crew_theme::theme();
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));
    buf.set_line(
        0,
        rows - 1,
        &Line::styled(
            "budget exceeded \u{2014} swarm cancelled",
            style(t.status_fg, true),
        ),
        cols,
    );
    let mut cells = crate::tui::to_cells(&buf);
    // to_cells is origin-relative to the whole buffer; keep only the notice row.
    cells.retain(|c| c.row == rows - 1);
    cells
}
