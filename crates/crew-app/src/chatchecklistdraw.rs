//! Drawing the plan rows (`chatchecklist`) and the code line under them.
//!
//! A step reads in the swarm rows' marks, so a plan moving looks the same
//! whichever engine runs it: `●` in the accent on the step the agent is on,
//! `✓` on the ones it finished, `○` on the rest. Weight carries the eye:
//! the step it is on is bold, what is left is plain ink, and what is done
//! falls back to the muted ink, because it is history.
use crew_hive::tools::todowire::{Step, StepState};
use crew_hive::TaskState;
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatchecklist::{tally, window, Row};
use crate::chatswarmcell::push_styled;
use crate::chatwidth::str_w;

/// The column the plan starts in, the code line's.
const INSET: u16 = 1;

/// The live region `top..top + n`: the plan's rows, then the code line on
/// the last. One row is the code line alone, exactly what it was before
/// there were plans.
pub(crate) fn live_cells(pane: &ChatPane, cols: u16, top: u16, n: u16) -> Vec<CellView> {
    if n == 0 {
        return Vec::new();
    }
    let line = top + n - 1;
    let mut out = crate::chatcodeline::line_cells(pane, cols, line);
    let r = crate::chatchecklist::plan_space(n - 1, pane.checklist.len());
    if r > 0 && crate::chatcodeline::code_rows(pane, cols) > 0 {
        out.extend(plan_cells(&pane.checklist, cols, line - r, r));
    }
    out
}

/// The plan's `r` rows from `top`, with the tally at the edge of the first.
pub(crate) fn plan_cells(steps: &[Step], cols: u16, top: u16, r: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    let end = cols.saturating_sub(1);
    let count = tally(steps);
    // The tally takes the first row's edge when the row still has room for
    // a step's words beside it; on a narrow pane the steps keep the width.
    let tally_w = str_w(&count) as u16;
    let tally_at = end.saturating_sub(tally_w);
    let with_tally = tally_at > INSET + 12;
    for (k, row) in window(steps, usize::from(r)).into_iter().enumerate() {
        let y = top + k as u16;
        let stop = if k == 0 && with_tally {
            tally_at - 1
        } else {
            end
        };
        let mut col = INSET;
        match row {
            Row::Step(i) => {
                let s = &steps[i];
                let (glyph, gc, _) = crate::swarm::view::state_style(task_state(s.state));
                let (ink, bold) = match s.state {
                    StepState::Active => (t.ink, true),
                    StepState::Pending => (t.ink, false),
                    StepState::Done => (t.text_muted, false),
                };
                push_styled(&mut out, &mut col, y, [(glyph, gc), (' ', gc)], stop, bold);
                let words = crate::chatwidth::clip_w(&s.text, usize::from(stop - col));
                push_styled(
                    &mut out,
                    &mut col,
                    y,
                    words.chars().map(|c| (c, ink)),
                    stop,
                    bold,
                );
            }
            Row::Done(n) | Row::Earlier(n) | Row::More(n) => {
                let words = match row {
                    Row::Done(_) => format!("\u{2713} {n} done"),
                    Row::Earlier(_) => format!("+{n} earlier"),
                    _ => format!("+{n} more"),
                };
                let muted = t.text_muted;
                push_styled(
                    &mut out,
                    &mut col,
                    y,
                    words.chars().map(|c| (c, muted)),
                    stop,
                    false,
                );
            }
        }
        if k == 0 && with_tally {
            let mut at = tally_at;
            push_styled(
                &mut out,
                &mut at,
                y,
                count.chars().map(|c| (c, t.text_muted)),
                end,
                false,
            );
        }
    }
    out
}

/// A step's state in the swarm rows' terms, whose marks it borrows.
fn task_state(s: StepState) -> TaskState {
    match s {
        StepState::Pending => TaskState::Pending,
        StepState::Active => TaskState::Running,
        StepState::Done => TaskState::Done,
    }
}

#[cfg(test)]
#[path = "chatchecklistdraw_tests.rs"]
mod tests;
