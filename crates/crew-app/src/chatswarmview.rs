//! Draws the live swarm-run status line at the bottom of the chat message
//! area: one row saying what crew is doing right now — a spinner, the running
//! task's title, its elapsed time, and how much of the plan has settled. The
//! plan itself is not shown live; it lands in the transcript when the run
//! folds (`chatswarmrec`). State lives in `chatswarm`.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatswarm::{SwarmStatus, SwarmTask};
use crew_hive::TaskState;

/// Shown when the plan has arrived but nothing is running yet — the gap
/// before the first `AgentSpawned`, and the gap between one task settling and
/// the next spawning.
const WORKING: &str = "Working…";
/// Below this width the elapsed column is dropped (the title needs the room).
/// The counter is never dropped: it's the reason the line exists.
const ELAPSED_MIN_COLS: u16 = 16;

/// Rows the live line occupies in the message area (0 = no live run).
pub(crate) fn swarm_rows(pane: &ChatPane, _rows: u16) -> u16 {
    match &pane.swarm {
        Some(s) if !s.tasks.is_empty() => 1,
        _ => 0,
    }
}

/// The task the line names: the oldest `Running` one, plus how many others are
/// running alongside it. `None` when nothing is running.
///
/// Oldest rather than newest so the line stays put under parallelism — naming
/// the most recent spawn would make it flicker between tasks as agents come
/// and go. A `Running` task always has `started` stamped (`chatswarm::apply`),
/// but unstamped ones sort last so a stamped task still wins if that changes.
fn focus(s: &SwarmStatus) -> Option<(&SwarmTask, usize)> {
    let mut running: Vec<&SwarmTask> = s
        .tasks
        .iter()
        .filter(|t| t.state == TaskState::Running)
        .collect();
    running.sort_by_key(|t| (t.started.is_none(), t.started));
    let first = *running.first()?;
    Some((first, running.len() - 1))
}

fn push_str(v: &mut Vec<CellView>, col: &mut u16, row: u16, s: &str, fg: (u8, u8, u8)) {
    for c in s.chars() {
        // Advance by display width (a wide CJK/emoji glyph occupies two
        // cells) so text after a wide glyph doesn't overlap it; zero-width
        // marks are skipped like `chatwidth::place_row` does.
        let w = crate::chatwidth::char_w(c) as u16;
        if w == 0 {
            continue;
        }
        v.push(CellView {
            col: *col,
            row,
            c,
            fg,
            bg: crew_theme::theme().page_bg,
            bold: false,
            italic: false,
        });
        *col += w;
    }
}

/// Render the status line at `top_row`. `now_ms` drives the spinner (0 in
/// tests = first frame, and suppresses elapsed so tests stay deterministic).
pub(crate) fn block_cells(pane: &ChatPane, cols: u16, top_row: u16, now_ms: u64) -> Vec<CellView> {
    let Some(s) = &pane.swarm else {
        return Vec::new();
    };
    if s.tasks.is_empty() {
        return Vec::new();
    }
    let theme = crew_theme::theme();
    let mut v = Vec::new();

    let (done, total) = s.settled();
    let counter = format!("{done}/{total}");
    let focused = focus(s);
    let title_src = match focused {
        Some((t, _)) => t.title.as_str(),
        None => WORKING,
    };
    // ` +N` marks parallel work. It is not clamp-able below — it's the only
    // signal that other tasks are running, so the title yields columns to it.
    let suffix = match focused {
        Some((_, others)) if others > 0 => format!(" +{others}"),
        _ => String::new(),
    };
    // Elapsed derives from `started` at render time — the per-frame redraw
    // while busy animates it for free — and is gated on `now_ms != 0` so tests
    // that don't care (now_ms == 0) stay deterministic.
    let elapsed = focused
        .filter(|_| now_ms != 0)
        .and_then(|(t, _)| t.started)
        .map(|s| format!("{}s", s.elapsed().as_secs()))
        .filter(|_| cols >= ELAPSED_MIN_COLS);

    let f = (now_ms / 120) as usize % crate::update::SPINNER.len();
    let mut col = 1u16;
    push_str(
        &mut v,
        &mut col,
        top_row,
        &crate::update::SPINNER[f].to_string(),
        crate::palette::accent(),
    );
    push_str(&mut v, &mut col, top_row, " ", crate::palette::accent());

    // Reserve room for the right-aligned columns. The counter always shows;
    // elapsed drops below ELAPSED_MIN_COLS. Each column claims exactly
    // `len + 1` — the same budget `next_start` charges below.
    let mut reserve = 1u16 + counter.len() as u16 + 1;
    if let Some(e) = &elapsed {
        reserve += e.len() as u16 + 1;
    }
    // Columns left for the title and its suffix. The suffix is preferred over
    // the title — it's the only signal parallel work exists — but on a pane
    // with room for neither it drops too, rather than overrunning `reserve`
    // and landing on the counter.
    let avail = cols.saturating_sub(col + reserve);
    let suffix_w = crate::chatwidth::str_w(&suffix) as u16;
    let (suffix, suffix_w) = if suffix_w <= avail {
        (suffix, suffix_w)
    } else {
        (String::new(), 0)
    };
    let max_title = (avail - suffix_w) as usize;
    // Display-width-aware clamp: `.chars().take(n)` counts chars, so a
    // CJK/emoji title (2 display columns per glyph) could select twice as many
    // columns as `max_title` allows, colliding with the elapsed column.
    let title_chars: Vec<char> = title_src.chars().collect();
    let title_end = crate::chatwidth::fit_end(&title_chars, 0, max_title);
    let title: String = title_chars[..title_end].iter().collect();
    push_str(&mut v, &mut col, top_row, &title, theme.text_muted);
    push_str(&mut v, &mut col, top_row, &suffix, theme.text_muted);

    // Right-aligned from the pane edge, each exactly `len + 1` inside whatever
    // sits to its right — the same per-column budget `reserve` charged above,
    // so title and columns can never collide (an extra -1 here once
    // double-billed the gap and overlapped the title):
    // title ... elapsed ... counter.
    let next_start = cols.saturating_sub(counter.len() as u16 + 1);
    let mut ccol = next_start;
    push_str(&mut v, &mut ccol, top_row, &counter, theme.text_muted);
    if let Some(e) = &elapsed {
        let mut ecol = next_start.saturating_sub(e.len() as u16 + 1);
        push_str(&mut v, &mut ecol, top_row, e, theme.text_muted);
    }
    v
}

#[cfg(test)]
#[path = "chatswarmview_tests.rs"]
mod tests;
