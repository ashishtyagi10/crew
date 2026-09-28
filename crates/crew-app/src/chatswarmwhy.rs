//! Why a task failed, said under its row: the first line of the provider's
//! words (`SwarmTask::why`), indented to the row's words and in the ✗'s red.
//!
//! A red ✗ alone reads the same for a timeout, a rejected key and a model the
//! host does not serve, and those want different answers — wait, fix the
//! key, pick another model. The hive already hands the reason over (the
//! agent's `Failed` event); this puts it where the eye already is. The
//! record the block folds into carries the same line, so the reason is still
//! there after the rows stop moving.
use crew_hive::TaskState;
use crew_render::CellView;

use crate::chatswarm::{SwarmStatus, SwarmTask};
use crate::chatwidth::clip_words;

/// Widest a reason runs in the record card. The card wraps at the pane, and
/// a provider's first line fits well inside this; what does not is a body
/// nobody boiled down, which the LOG keeps whole.
const RECORD_W: usize = 96;

/// The first line of a failed task's reason, or `None` while it has not
/// failed or never said why — the rest is the provider's hint or body, and
/// one line is what a row can spare.
pub(crate) fn why(t: &SwarmTask) -> Option<&str> {
    if t.state != TaskState::Failed {
        return None;
    }
    t.why
        .lines()
        .next()
        .map(str::trim)
        .filter(|l| !l.is_empty())
}

/// The reason's row: from the row's words at `text_start` to the pane's
/// margin, clipped at a word, in the failure's ink. Nothing when no reason,
/// or when the pane leaves fewer columns than a title would get.
pub(crate) fn cells(t: &SwarmTask, text_start: u16, cols: u16, row: u16) -> Vec<CellView> {
    let mut v = Vec::new();
    let end = cols.saturating_sub(1);
    let w = usize::from(end.saturating_sub(text_start));
    let Some(why) = why(t).filter(|_| w >= 4) else {
        return v;
    };
    let (_, ink, _) = crate::swarm::view::state_style(TaskState::Failed);
    let mut col = text_start;
    crate::chatswarmcell::push_str(&mut v, &mut col, row, &clip_words(why, w), ink, end);
    v
}

/// The record's line under the `i`th task's row, indented under its words
/// the way the live block drew it; `None` when the task has no reason.
///
/// The card renders markdown, which strips a line's leading spaces — the
/// reason landed flush left, under the numbers, reading as a row of its own.
/// So the line opens with the one plain space every row opens with (and
/// loses) and indents with no-break spaces, which the renderer keeps.
pub(crate) fn record_line(s: &SwarmStatus, i: usize) -> Option<String> {
    let why = why(&s.tasks[i])?;
    let numw = s.tasks.len().to_string().len();
    let indent = "\u{a0}".repeat(numw + 3);
    Some(format!(" {indent}{}", clip_words(why, RECORD_W)))
}

#[cfg(test)]
#[path = "chatswarmwhy_tests.rs"]
mod tests;
