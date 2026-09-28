//! What each finished task spent, at its row's right end: `↑1.2k ↓340`, the
//! status line's spend wording one task wide, so the row that ate the budget
//! stands out from the ones that did not. The line above sums the run; only
//! the rows can say which task it went on.
//!
//! A running task's count is left off: it changes every frame, and the
//! status line already carries the live total.
use crew_render::CellView;

use crate::chathdr::fmt_tokens;
use crate::chatswarm::{terminal, SwarmStatus, SwarmTask};
use crate::chatwidth::str_w;

/// `↑1.2k ↓340` — input then output tokens, the one spend wording the status
/// line and the rows share.
pub(crate) fn words(input: u64, output: u64) -> String {
    format!(
        "\u{2191}{} \u{2193}{}",
        fmt_tokens(input),
        fmt_tokens(output)
    )
}

/// The task's spend as its row says it, once it has stopped moving and only
/// when it spent anything — a task that never reached a model has no count
/// worth a column.
pub(crate) fn of(t: &SwarmTask) -> Option<String> {
    (terminal(t.state) && t.tokens_in + t.tokens_out > 0).then(|| words(t.tokens_in, t.tokens_out))
}

/// The count column's width over the first `shown` rows: the widest count
/// among them, 0 when none has one.
pub(crate) fn col_w(s: &SwarmStatus, shown: usize) -> u16 {
    s.tasks[..shown]
        .iter()
        .filter_map(of)
        .map(|c| str_w(&c) as u16)
        .max()
        .unwrap_or(0)
}

/// The row's count, right-aligned against `end` (one past its last column),
/// in the meta ink the status line's trailer wears.
pub(crate) fn cells(t: &SwarmTask, end: u16, row: u16) -> Vec<CellView> {
    let mut v = Vec::new();
    if let Some(cost) = of(t) {
        let mut col = end.saturating_sub(str_w(&cost) as u16);
        let muted = crew_theme::theme().text_muted;
        crate::chatswarmcell::push_str(&mut v, &mut col, row, &cost, muted, end);
    }
    v
}

#[cfg(test)]
#[path = "chatswarmcost_tests.rs"]
mod tests;
