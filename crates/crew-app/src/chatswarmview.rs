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
/// Below this width the elapsed column is dropped too. Narrower than
/// `TOKENS_MIN_COLS` so tokens drop first as the pane shrinks, and elapsed —
/// the more at-a-glance-useful of the two for a running task — survives
/// longer.
const ELAPSED_MIN_COLS: u16 = 16;

/// Rows the live block occupies in the message area (0 = no live run).
pub(crate) fn swarm_rows(pane: &ChatPane, _rows: u16) -> u16 {
    match &pane.swarm {
        Some(s) => (s.tasks.len() as u16).min(MAX_ROWS),
        None => 0,
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
        // Title, clamped to leave room for the elapsed/token columns (or the
        // edge). Elapsed derives from `started` at render time — the
        // per-frame redraw while busy animates it for free — and is gated on
        // `now_ms != 0` so tests that don't care (now_ms == 0) stay
        // deterministic (`chatview::agent_state_str` is the pattern this
        // imitates).
        let elapsed = (t.state == TaskState::Running && now_ms != 0)
            .then(|| t.started.map(|s| format!("{}s", s.elapsed().as_secs())))
            .flatten()
            .filter(|_| cols >= ELAPSED_MIN_COLS);
        let tok =
            (t.tokens > 0 && cols >= TOKENS_MIN_COLS).then(|| crate::chatswarm::fmt_tok(t.tokens));
        // Width rule: tokens drop first (at `TOKENS_MIN_COLS`), elapsed
        // survives to `ELAPSED_MIN_COLS`, then both drop on very narrow
        // panes. Reserve room for whichever of the two are shown.
        let mut reserve = 1u16;
        if let Some(e) = &elapsed {
            reserve += e.len() as u16 + 1;
        }
        if let Some(tk) = &tok {
            reserve += tk.len() as u16 + 1;
        }
        let max_title = cols.saturating_sub(col + reserve) as usize;
        // Display-width-aware clamp: `.chars().take(n)` counts chars, so a
        // CJK/emoji title (2 display columns per glyph) could select twice
        // as many columns as `max_title` allows, colliding with the token
        // column (or the pane edge on narrow panes).
        let title_chars: Vec<char> = t.title.chars().collect();
        let title_end = crate::chatwidth::fit_end(&title_chars, 0, max_title);
        let title: String = title_chars[..title_end].iter().collect();
        push_str(&mut v, &mut col, row, &title, theme.text_muted);
        // Right-aligned from the pane edge, each with a 1-column gap from
        // whatever sits to its right: tokens outermost, elapsed just inside
        // it (title ... elapsed ... tokens).
        let mut next_start = cols;
        if let Some(tok) = &tok {
            let tok_start = next_start.saturating_sub(tok.len() as u16 + 1);
            let mut tcol = tok_start;
            push_str(&mut v, &mut tcol, row, tok, theme.text_muted);
            next_start = tok_start.saturating_sub(1);
        }
        if let Some(e) = &elapsed {
            let mut ecol = next_start.saturating_sub(e.len() as u16 + 1);
            push_str(&mut v, &mut ecol, row, e, theme.text_muted);
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
