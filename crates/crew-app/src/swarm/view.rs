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

    // HUD row: live/done/failed + cost in dollars.
    let totals = fleet.totals();
    let hud = format!(
        " live:{} done:{} failed:{} cost:${:.4}",
        totals.live,
        totals.done,
        totals.failed,
        totals.micros_usd as f64 / 1_000_000.0,
    );
    buf.set_line(0, 0, &Line::styled(hud, style(t.ink, false)), cols);

    // Task rows below the HUD. A task with no spawned agent yet is Pending.
    let by_task: HashMap<_, _> = fleet.agents().map(|a| (a.task, a)).collect();
    let avail = rows.saturating_sub(1) as usize;
    let tasks = graph.tasks();
    // Keep one row for the overflow note when the list doesn't fit.
    let shown = if tasks.len() > avail {
        avail.saturating_sub(1)
    } else {
        tasks.len()
    };
    for (i, spec) in tasks.iter().take(shown).enumerate() {
        let agent = by_task.get(&spec.id);
        let state = agent.map_or(TaskState::Pending, |a| a.state);
        let (glyph, color, bold) = state_style(state);
        let mut spans = vec![
            Span::styled(format!(" {glyph} "), style(color, bold)),
            Span::styled(spec.title.clone(), style(color, bold)),
        ];
        // The live tail: what the agent last printed (or the failure reason).
        let tail = agent
            .filter(|_| matches!(state, TaskState::Running | TaskState::Failed))
            .map(|a| a.last_line.as_str())
            .unwrap_or_default();
        if !tail.is_empty() {
            spans.push(Span::styled(
                format!(" \u{2014} {tail}"),
                style(t.text_muted, false),
            ));
        }
        buf.set_line(0, (i + 1) as u16, &Line::from(spans), cols);
    }
    if tasks.len() > shown && avail > 0 {
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
