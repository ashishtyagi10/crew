//! The task rows under the swarm block's status line: one per planned task —
//! its number, state glyph, specialist, title, what it waits on, and its span
//! on the run's clock (`chatswarmspan`) — so the pane shows the PLAN moving,
//! not just how much of it has stopped.
//!
//! Rows sit in plan order, the order `swarm/compose` lists them in, and a
//! task's dependencies are named by the numbers of the rows they are — `← 1,2`
//! — so a reader sees the diamond without a graph. At most [`MAX_ROWS`] rows
//! are drawn; a longer plan ends in `… +N more tasks`. Under width pressure
//! the block gives up its bar, then every row's specialist, then every row's
//! deps, and only then clips titles: the name of the work is the last thing
//! to go, and the rows give things up TOGETHER — one row keeping a column
//! its neighbour lost reads as a different layout, not a tighter one.
use crew_hive::TaskState;
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatswarm::SwarmStatus;
use crate::chatswarmcell::push_styled;
use crate::chatwidth::{clip_w, str_w};
use crate::shimmer::Color;

/// Rows the list may take under the status line, the tail row included.
pub(crate) const MAX_ROWS: usize = 8;
/// Left inset, under the status line's spinner.
const INSET: u16 = 1;

/// How many of `n` tasks get a row, and how many the tail names instead:
/// all of them when they fit, else one row fewer to make room for the tail.
pub(crate) fn shown(n: usize) -> (usize, usize) {
    if n > MAX_ROWS {
        (MAX_ROWS - 1, n - (MAX_ROWS - 1))
    } else {
        (n, 0)
    }
}

/// `… +3 more tasks` — the row that stands in for the ones not drawn, its
/// mark under the numbers.
pub(crate) fn tail(hidden: usize) -> String {
    format!("\u{2026} +{}", crate::wording::count(hidden, "more task"))
}

/// Rows the block wants under its status line for this plan.
pub(crate) fn rows_wanted(s: &SwarmStatus) -> u16 {
    let (shown, hidden) = shown(s.tasks.len());
    (shown + usize::from(hidden > 0)) as u16
}

/// The number the plan's `i`th task goes by on its row and in others' deps.
fn number(s: &SwarmStatus, id: crew_hive::TaskId) -> Option<usize> {
    s.tasks.iter().position(|t| t.id == id).map(|p| p + 1)
}

/// The `i`th task's words at a richness `level`: `[specialist, title, deps]`
/// whole at 0, without the specialist at 1, the title alone at 2 — and the
/// title clipped to `w` when even that is too long.
pub(crate) fn words(s: &SwarmStatus, i: usize, w: usize, level: u8) -> (String, String, String) {
    let t = &s.tasks[i];
    let deps: Vec<String> = t
        .deps
        .iter()
        .filter_map(|d| number(s, *d))
        .map(|n| n.to_string())
        .collect();
    let deps = if deps.is_empty() || level >= 2 {
        String::new()
    } else {
        format!(" \u{2190} {}", deps.join(","))
    };
    let spec = if t.specialty.is_empty() || level >= 1 {
        String::new()
    } else {
        format!("{}  ", t.specialty)
    };
    let title = clip_w(&t.title, w.saturating_sub(str_w(&spec) + str_w(&deps)));
    (spec, title, deps)
}

/// The richest level at which every one of the first `shown` rows fits `w`
/// whole (no title clipped), or 2 when none does.
pub(crate) fn level(s: &SwarmStatus, shown: usize, w: usize) -> u8 {
    (0..2u8)
        .find(|&l| (0..shown).all(|i| !words(s, i, w, l).1.ends_with('\u{2026}')))
        .unwrap_or(2)
}

/// The title's colour: the state's while it moves or fails, ink once done,
/// muted while it waits — the glyph's vocabulary (`swarm/view`), one row wide.
fn title_color(state: TaskState, t: &crew_theme::Theme) -> (Color, bool) {
    match state {
        TaskState::Running | TaskState::Failed => {
            let (_, c, bold) = crate::swarm::view::state_style(state);
            (c, bold)
        }
        TaskState::Done => (t.ink, false),
        TaskState::Pending | TaskState::Ready | TaskState::Cancelled => (t.text_muted, false),
    }
}

/// The rows, from `top` down, for a `cols`-wide block at `now_ms`.
pub(crate) fn cells(pane: &ChatPane, cols: u16, top: u16, now_ms: u64) -> Vec<CellView> {
    let Some(s) = pane.swarm.as_ref() else {
        return Vec::new();
    };
    let t = crew_theme::theme();
    let muted = t.text_muted;
    let (shown, hidden) = shown(s.tasks.len());
    let numw = s.tasks.len().to_string().len() as u16;
    let text_start = INSET + numw + 3; // `N ● ` after the inset
    let bar_w = crate::chatswarmspan::bar_cols(cols);
    let bar_start = cols.saturating_sub(1 + bar_w);
    // The text ends a column short of the bar (or of the pane's margin).
    let text_end = if bar_w > 0 {
        bar_start - 2
    } else {
        cols.saturating_sub(1)
    };
    let text_w = usize::from(text_end.saturating_sub(text_start));
    let axis = crate::chatswarmspan::axis(&s.tasks, now_ms);
    let level = level(s, shown, text_w);
    let mut v = Vec::new();
    for i in 0..shown {
        let task = &s.tasks[i];
        let row = top + i as u16;
        let (glyph, gc, gbold) = crate::swarm::view::state_style(task.state);
        let mut col = INSET;
        let num = format!("{:>w$} ", i + 1, w = usize::from(numw));
        push_styled(
            &mut v,
            &mut col,
            row,
            num.chars().map(|c| (c, muted)),
            cols,
            false,
        );
        push_styled(&mut v, &mut col, row, [(glyph, gc), (' ', gc)], cols, gbold);
        if text_w >= 4 {
            let (spec, title, deps) = words(s, i, text_w, level);
            let (tc, tbold) = title_color(task.state, t);
            push_styled(
                &mut v,
                &mut col,
                row,
                spec.chars().map(|c| (c, muted)),
                text_end,
                false,
            );
            push_styled(
                &mut v,
                &mut col,
                row,
                title.chars().map(|c| (c, tc)),
                text_end,
                tbold,
            );
            push_styled(
                &mut v,
                &mut col,
                row,
                deps.chars().map(|c| (c, muted)),
                text_end,
                false,
            );
        }
        if let (Some(axis), true) = (axis, bar_w > 0) {
            let bar = crate::chatswarmspan::bar(task, axis, bar_w, now_ms, gc, t.border_normal);
            let mut bcol = bar_start;
            push_styled(&mut v, &mut bcol, row, bar, cols, false);
        }
    }
    if hidden > 0 {
        let mut col = INSET;
        let line = tail(hidden);
        let row = top + shown as u16;
        push_styled(
            &mut v,
            &mut col,
            row,
            line.chars().map(|c| (c, muted)),
            cols,
            false,
        );
    }
    v
}

#[cfg(test)]
#[path = "chatswarmrows_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "chatswarmrowsfit_tests.rs"]
mod fit_tests;
