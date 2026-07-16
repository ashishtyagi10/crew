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
/// Columns claimed by the fixed left prefix — a 1-column margin, the
/// spinner, and one space — before the title/suffix region begins. Anything
/// placed right-aligned from the pane edge (counter, elapsed) is floored
/// here so it can never be pushed left into the prefix, however narrow the
/// pane gets.
const PREFIX_END: u16 = 3;

/// Whether `cols` has room for the line at all: the prefix plus the counter,
/// with no gap between them. Below this there's nothing sensible left to
/// draw, so `block_cells` returns nothing — this must agree with that so the
/// row budget and the draw never disagree.
fn line_fits(pane: &ChatPane, cols: u16) -> bool {
    let Some(s) = &pane.swarm else {
        return false;
    };
    if s.tasks.is_empty() {
        return false;
    }
    let (done, total) = s.settled();
    cols >= PREFIX_END + format!("{done}/{total}").len() as u16
}

/// Rows the live line occupies in the message area (0 = no live run, or a
/// pane too narrow to show the line without the counter colliding with the
/// spinner — see `line_fits`).
pub(crate) fn swarm_rows(pane: &ChatPane, cols: u16) -> u16 {
    if line_fits(pane, cols) {
        1
    } else {
        0
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

/// Places `s` on `row` starting at `*col`, advancing by display width and
/// never emitting a cell at or beyond `max_col` — delegated to
/// `chatwidth::place_row`, which already advances by `char_w` and skips
/// zero-width marks, so `max_col` is enforced structurally rather than
/// relying on the caller's arithmetic to have pre-sized `s` correctly (a
/// wide glyph that would straddle past `max_col` simply isn't drawn, instead
/// of the old `fit_end`-based clamp that could force one char through even
/// at a zero-column budget).
fn push_str(
    v: &mut Vec<CellView>,
    col: &mut u16,
    row: u16,
    s: &str,
    fg: (u8, u8, u8),
    max_col: u16,
) {
    let bg = crew_theme::theme().page_bg;
    *col = crate::chatwidth::place_row(*col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        v.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold: false,
            italic: false,
        });
    });
}

/// Render the status line at `top_row`. `now_ms` drives the spinner (0 in
/// tests = first frame, and suppresses elapsed so tests stay deterministic).
pub(crate) fn block_cells(pane: &ChatPane, cols: u16, top_row: u16, now_ms: u64) -> Vec<CellView> {
    let Some(s) = &pane.swarm else {
        return Vec::new();
    };
    if !line_fits(pane, cols) {
        // Genuinely no room, even for the prefix + counter alone — drop the
        // whole line rather than emit anything that could overlap.
        // `swarm_rows` runs the identical check so the row budget agrees.
        return Vec::new();
    }
    let theme = crew_theme::theme();
    let mut v = Vec::new();

    let (done, total) = s.settled();
    let counter = format!("{done}/{total}");
    let counter_w = counter.len() as u16;
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
        cols,
    );
    push_str(
        &mut v,
        &mut col,
        top_row,
        " ",
        crate::palette::accent(),
        cols,
    );
    // `line_fits` guaranteed `cols` has room for the prefix in full, so `col`
    // is now exactly `PREFIX_END` — the leftmost column anything to the
    // right may use.

    // Counter: right-aligned at the pane edge, floored at `col` so it can
    // never be pushed left into the prefix on a narrow pane — the bug this
    // replaces let pure right-edge arithmetic decide, and on a narrow enough
    // pane it decided to land there.
    let counter_start = cols.saturating_sub(counter_w).max(col);

    // Elapsed sits one gap column left of the counter — but only if that
    // doesn't run it back into the prefix/title area; otherwise it's
    // dropped, same as the `ELAPSED_MIN_COLS` gate above already mostly
    // ensures.
    let elapsed_start = elapsed.as_ref().and_then(|e| {
        let start = counter_start.saturating_sub(1 + e.len() as u16);
        (start >= col).then_some(start)
    });
    let elapsed = elapsed.filter(|_| elapsed_start.is_some());

    // Columns left for the title + suffix: everything up to one gap column
    // before whichever of {elapsed, counter} sits leftmost. Unlike the old
    // `reserve`/`next_start` pair — two separate expressions of the same
    // budget that could disagree — this is just the gap between two column
    // positions computed once, so there's nothing left to double-book.
    let region_end = elapsed_start.unwrap_or(counter_start);
    let title_limit = region_end.saturating_sub(1).max(col);
    let avail = title_limit - col;
    let suffix_w = crate::chatwidth::str_w(&suffix) as u16;
    // The suffix is preferred over the title — it's the only signal parallel
    // work exists — but on a pane with room for neither it drops too, rather
    // than overrunning into the counter/elapsed region.
    let (suffix, suffix_w) = if suffix_w <= avail {
        (suffix, suffix_w)
    } else {
        (String::new(), 0)
    };
    push_str(
        &mut v,
        &mut col,
        top_row,
        title_src,
        theme.text_muted,
        title_limit - suffix_w,
    );
    push_str(
        &mut v,
        &mut col,
        top_row,
        &suffix,
        theme.text_muted,
        title_limit,
    );

    let mut ccol = counter_start;
    push_str(&mut v, &mut ccol, top_row, &counter, theme.text_muted, cols);
    if let (Some(e), Some(start)) = (&elapsed, elapsed_start) {
        let mut ecol = start;
        push_str(
            &mut v,
            &mut ecol,
            top_row,
            e,
            theme.text_muted,
            counter_start.saturating_sub(1),
        );
    }
    v
}

#[cfg(test)]
#[path = "chatswarmview_tests.rs"]
mod tests;
