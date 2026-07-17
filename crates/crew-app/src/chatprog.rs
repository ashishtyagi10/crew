//! The live swarm run's progress bar: one row directly above the composer,
//! tracking how many of the plan's tasks have settled. Replaces the old
//! in-pane rain patch, which drew over the composer's own text — this claims
//! a row instead of overlaying one, the same way `chatqueue`'s indicator does,
//! so the message-area budget stays honest.
//!
//! The bar is determinate: a swarm plan has a known task count, so there's no
//! reason to show an indeterminate animation. Terminal states (done, failed,
//! cancelled) all count as settled — the bar tracks "how much of the plan is
//! still moving", not "how much succeeded". That distinction — which tasks
//! settled how — lives only in the folded transcript record
//! (`chatswarmrec`) once the run ends; the live view no longer shows it.
use crew_render::CellView;

use crate::chat::ChatPane;

/// Narrowest bar still worth drawing. Below this the row is dropped entirely
/// rather than drawn as an unreadable stub.
const MIN_BAR: u16 = 4;
/// Left inset, matching the swarm block and the queued indicator.
const INSET: u16 = 1;

/// The bar's geometry for `cols`, or `None` when there's no live run (or no
/// room). Both [`progress_rows`] and [`bar_cells`] route through this, so the
/// row a pane budgets and the row it draws can never disagree.
///
/// The bar's width mirrors the status line's left "words" (`chatswarmview::
/// words_width`) rather than the whole pane, so it underlines the text instead
/// of stretching edge to edge. `now_ms` feeds that width because the words
/// carry a live elapsed clock; `progress_rows` passes 0 (existence is
/// clock-independent — the words are always at least a counter wide).
fn geom(pane: &ChatPane, cols: u16, now_ms: u64) -> Option<(usize, usize, u16)> {
    let s = pane.swarm.as_ref()?;
    let (done, total) = s.settled();
    if total == 0 {
        return None;
    }
    // Match the words; never exceed the pane (the last column is a margin).
    let words = crate::chatswarmview::words_width(pane, cols, now_ms)?;
    let bar_w = words.min(cols.saturating_sub(INSET));
    (bar_w >= MIN_BAR).then_some((done, total, bar_w))
}

/// Rows the progress bar claims above the composer: 1 during a live swarm run
/// that fits, else 0. Clock-independent, so it passes `now_ms = 0`.
pub(crate) fn progress_rows(pane: &ChatPane, cols: u16) -> u16 {
    geom(pane, cols, 0).is_some() as u16
}

/// Render the bar at `row`: filled cells in the accent, the remainder muted.
/// The `done/total` count lives on the status line above (`chatswarmview`).
/// `now_ms` sizes the bar to that line's live-elapsed width.
pub(crate) fn bar_cells(pane: &ChatPane, cols: u16, row: u16, now_ms: u64) -> Vec<CellView> {
    let Some((done, total, bar_w)) = geom(pane, cols, now_ms) else {
        return Vec::new();
    };
    let theme = crew_theme::theme();
    // Integer floor: the bar only fills completely once every task has
    // settled, so a full bar always means a finished plan.
    let filled = (done * bar_w as usize / total) as u16;
    let mut cells = Vec::new();
    let mut push = |col: u16, c: char, fg: (u8, u8, u8), bold: bool| {
        cells.push(CellView {
            col,
            row,
            c,
            fg,
            bg: theme.page_bg,
            bold,
            italic: false,
        })
    };
    for i in 0..bar_w {
        let on = i < filled;
        let (c, fg) = if on {
            ('\u{2588}', crate::palette::accent())
        } else {
            ('\u{2591}', theme.text_muted)
        };
        push(INSET + i, c, fg, on);
    }
    cells
}

#[cfg(test)]
#[path = "chatprog_tests.rs"]
mod tests;
