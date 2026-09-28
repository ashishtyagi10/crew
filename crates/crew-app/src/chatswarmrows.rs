//! The task rows under the swarm block's status line: one per planned task —
//! its number, state glyph, specialist, title, what it waits on, its span on
//! the run's clock (`chatswarmspan`) and, once it finished, what it spent
//! (`chatswarmcost`) — so the pane shows the PLAN moving, not just how much
//! of it has stopped. A failed task says why on the line under it
//! (`chatswarmwhy`).
//!
//! Rows sit in plan order, the order `swarm/compose` lists them in, and a
//! task's dependencies are named by the numbers of the rows they are — `← 1,2`
//! — so a reader sees the diamond without a graph. Which rows fit is
//! `chatswarmplan`'s; a longer plan ends in `… +N more tasks`. Under width
//! pressure the block gives up its bar, then every row's count
//! (`chatswarmgeom`), then every row's specialist, then every row's deps, and
//! only then clips titles: the name of the work is the last thing to go, and
//! the rows give things up TOGETHER — one row keeping a column its neighbour
//! lost reads as a different layout, not a tighter one.
use crew_hive::TaskState;
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatswarm::SwarmStatus;
use crate::chatswarmcell::push_styled;
use crate::chatswarmgeom::INSET;
use crate::chatswarmplan::{shown, tail, Line};
use crate::chatwidth::{clip_w, str_w};
use crate::shimmer::Color;

/// Fewest columns a row's words are drawn in; below it the row is its
/// number and glyph alone.
pub(crate) const MIN_TEXT: usize = 4;

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
    // Padded to the widest specialist on screen: every title starts in one
    // column, not wherever its own specialist's name happened to end.
    let col = s.tasks[..shown(s).0]
        .iter()
        .map(|t| str_w(&t.specialty))
        .max();
    let spec = match col.unwrap_or(0) {
        c if c == 0 || level >= 1 => String::new(),
        c => format!("{}{}  ", t.specialty, " ".repeat(c - str_w(&t.specialty))),
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
    let numw = s.tasks.len().to_string().len();
    let g = crate::chatswarmgeom::geom(s, shown(s).0, cols);
    let axis = crate::chatswarmspan::axis(&s.tasks, now_ms);
    let mut v = Vec::new();
    for (r, line) in crate::chatswarmplan::lines(s).into_iter().enumerate() {
        let row = top + r as u16;
        let i = match line {
            Line::Task(i) => i,
            Line::Why(i) => {
                v.extend(crate::chatswarmwhy::cells(
                    &s.tasks[i],
                    g.text_start,
                    cols,
                    row,
                ));
                continue;
            }
            Line::Tail(hidden) => {
                let (mut col, text) = (INSET, tail(hidden));
                push_styled(
                    &mut v,
                    &mut col,
                    row,
                    text.chars().map(|c| (c, muted)),
                    cols,
                    false,
                );
                continue;
            }
        };
        let task = &s.tasks[i];
        let (glyph, gc, gbold) = crate::swarm::view::state_style(task.state);
        let mut col = INSET;
        let num = format!("{:>numw$} ", i + 1);
        push_styled(
            &mut v,
            &mut col,
            row,
            num.chars().map(|c| (c, muted)),
            cols,
            false,
        );
        push_styled(&mut v, &mut col, row, [(glyph, gc), (' ', gc)], cols, gbold);
        if g.text_w() >= MIN_TEXT {
            let (spec, title, deps) = words(s, i, g.text_w(), g.level);
            let (tc, tbold) = title_color(task.state, t);
            let end = g.text_end;
            push_styled(
                &mut v,
                &mut col,
                row,
                spec.chars().map(|c| (c, muted)),
                end,
                false,
            );
            push_styled(
                &mut v,
                &mut col,
                row,
                title.chars().map(|c| (c, tc)),
                end,
                tbold,
            );
            push_styled(
                &mut v,
                &mut col,
                row,
                deps.chars().map(|c| (c, muted)),
                end,
                false,
            );
        }
        if let (Some(axis), true) = (axis, g.bar_w > 0) {
            let bar = crate::chatswarmspan::bar(task, axis, g.bar_w, now_ms, gc, t.border_normal);
            let mut bcol = g.bar_start;
            push_styled(&mut v, &mut bcol, row, bar, cols, false);
        }
        if g.cost_w > 0 {
            v.extend(crate::chatswarmcost::cells(task, g.cost_end, row));
        }
    }
    v
}

#[cfg(test)]
#[path = "chatswarmrows_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "chatswarmrowsfit_tests.rs"]
mod fit_tests;
