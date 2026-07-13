//! Draws the live swarm-run block at the bottom of the chat message area:
//! one row per task — state glyph (running tasks animate a spinner), title,
//! right-aligned token count. State lives in `chatswarm`; when the run ends
//! the block folds into the transcript, so this only ever draws live runs.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatswarm::glyph;
use crew_hive::TaskState;

/// Most task rows the block will occupy; larger plans get a `… n more` row.
const MAX_ROWS: u16 = 8;
/// Below this width the token column is dropped (title needs the room).
const TOKENS_MIN_COLS: u16 = 24;

/// Rows the live block occupies in the message area (0 = no live run).
pub(crate) fn swarm_rows(pane: &ChatPane, _rows: u16) -> u16 {
    match &pane.swarm {
        Some(s) => (s.tasks.len() as u16).min(MAX_ROWS),
        None => 0,
    }
}

fn fmt_tok(n: u64) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

fn push_str(v: &mut Vec<CellView>, col: &mut u16, row: u16, s: &str, fg: (u8, u8, u8)) {
    for c in s.chars() {
        v.push(CellView {
            col: *col,
            row,
            c,
            fg,
            bg: crew_theme::theme().page_bg,
            bold: false,
            italic: false,
        });
        *col += 1;
    }
}

/// Render the block, one task per row starting at `top_row`. `now_ms` drives
/// the running-task spinner (0 in tests = first frame).
pub(crate) fn block_cells(pane: &ChatPane, cols: u16, top_row: u16, now_ms: u64) -> Vec<CellView> {
    let Some(s) = &pane.swarm else {
        return Vec::new();
    };
    let theme = crew_theme::theme();
    let mut v = Vec::new();
    let shown = (s.tasks.len()).min(MAX_ROWS as usize);
    // With more tasks than rows, the last row becomes the overflow summary.
    let listed = if s.tasks.len() > shown {
        shown - 1
    } else {
        shown
    };
    for (i, t) in s.tasks.iter().take(listed).enumerate() {
        let row = top_row + i as u16;
        let (g, fg) = match t.state {
            TaskState::Running => {
                let f = (now_ms / 120) as usize % crate::update::SPINNER.len();
                (crate::update::SPINNER[f], crate::palette::accent())
            }
            TaskState::Done => (glyph(&t.state), theme.activity),
            TaskState::Failed => (glyph(&t.state), theme.bell),
            _ => (glyph(&t.state), theme.text_muted),
        };
        let mut col = 1u16;
        push_str(&mut v, &mut col, row, &g.to_string(), fg);
        push_str(&mut v, &mut col, row, " ", fg);
        // Title, clamped to leave room for the token column (or the edge).
        let tok = (t.tokens > 0 && cols >= TOKENS_MIN_COLS).then(|| fmt_tok(t.tokens));
        let reserve = tok.as_ref().map(|s| s.len() as u16 + 2).unwrap_or(1);
        let max_title = cols.saturating_sub(col + reserve) as usize;
        let title: String = t.title.chars().take(max_title).collect();
        push_str(&mut v, &mut col, row, &title, theme.text_muted);
        if let Some(tok) = tok {
            let mut tcol = cols.saturating_sub(tok.len() as u16 + 1);
            push_str(&mut v, &mut tcol, row, &tok, theme.text_muted);
        }
    }
    if s.tasks.len() > shown {
        let more = s.tasks.len() - listed;
        let mut col = 1u16;
        push_str(
            &mut v,
            &mut col,
            top_row + listed as u16,
            &format!("… {more} more"),
            theme.text_muted,
        );
    }
    v
}

#[cfg(test)]
#[path = "chatswarmview_tests.rs"]
mod tests;
