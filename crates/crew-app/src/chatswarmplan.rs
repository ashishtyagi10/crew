//! Which rows the swarm block's list takes under its status line: one per
//! task in plan order, a reason line under each failed task that said why
//! (`chatswarmwhy`), and — when they do not all fit in [`MAX_ROWS`] — a tail
//! naming how many were left out. One list, read by both the row count the
//! pane budgets (`chatswarmview::swarm_rows`) and the rows drawn
//! (`chatswarmrows::cells`), so the two cannot disagree about a reason line.
use crate::chatswarm::SwarmStatus;

/// Rows the list may take under the status line, reason lines and the tail
/// included. A reason costs a row another task would have had: the block
/// stays the height it always was, and the tail still says what is missing.
pub(crate) const MAX_ROWS: usize = 8;

/// One row of the list, in the order drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Line {
    /// The `i`th task's row.
    Task(usize),
    /// The line under the `i`th task's row saying why it failed.
    Why(usize),
    /// `… +N more tasks`, for the `N` not drawn.
    Tail(usize),
}

/// Rows the `i`th task takes: its own, and its reason's when it has one.
fn need(s: &SwarmStatus, i: usize) -> usize {
    1 + usize::from(crate::chatswarmwhy::why(&s.tasks[i]).is_some())
}

/// How many tasks get a row, and how many the tail names instead: all of
/// them when they fit, else as many as fit in plan order with one row left
/// for the tail.
pub(crate) fn shown(s: &SwarmStatus) -> (usize, usize) {
    let n = s.tasks.len();
    if (0..n).map(|i| need(s, i)).sum::<usize>() <= MAX_ROWS {
        return (n, 0);
    }
    let mut used = 0;
    let k = (0..n)
        .take_while(|&i| {
            used += need(s, i);
            used < MAX_ROWS
        })
        .count();
    (k, n - k)
}

/// The list, top to bottom.
pub(crate) fn lines(s: &SwarmStatus) -> Vec<Line> {
    let (shown, hidden) = shown(s);
    let mut v = Vec::with_capacity(MAX_ROWS);
    for i in 0..shown {
        v.push(Line::Task(i));
        if need(s, i) > 1 {
            v.push(Line::Why(i));
        }
    }
    if hidden > 0 {
        v.push(Line::Tail(hidden));
    }
    v
}

/// Rows the block wants under its status line for this plan.
pub(crate) fn rows_wanted(s: &SwarmStatus) -> u16 {
    lines(s).len() as u16
}

/// `… +3 more tasks` — the row that stands in for the ones not drawn, its
/// mark under the numbers.
pub(crate) fn tail(hidden: usize) -> String {
    format!("\u{2026} +{}", crate::wording::count(hidden, "more task"))
}
