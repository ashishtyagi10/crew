//! The running agent's plan, drawn above the code line while its turn runs.
//!
//! Claude Code keeps a checklist with TodoWrite and crew's own agents with
//! `sys:todo`; both reach the pane as one `PluginEvent::Todo`, the whole list
//! each time. Until now the pane showed it as a tool line whose subject was a
//! JSON list, so the plan the agent was working to was the one thing on the
//! run nobody could read. Now it is the rows above the code line: what is
//! done, what the agent is on, what is left, and `2/5` at the edge.
//!
//! The rows belong to the live region the code line heads (`live_rows`), so
//! a pane short of room gives them up before it gives up the line. A list
//! longer than its rows keeps the step the agent is on and the one before it
//! in view, and counts the rest at either end.
use crew_hive::tools::todowire::{Step, StepState};

use crate::chat::ChatPane;

/// Rows the plan may take over the code line. Past this it would crowd the
/// reply it is the plan for.
pub(crate) const MAX_ROWS: usize = 5;

/// What one of the plan's rows shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    /// The list's `i`th step.
    Step(usize),
    /// `n` steps above the window, every one done: `✓ 3 done`.
    Done(usize),
    /// `n` steps above the window, not all done: `+3 earlier`.
    Earlier(usize),
    /// `n` steps below it: `+2 more`.
    More(usize),
}

/// Rows the live region wants: the code line's, and when the agent keeps a
/// plan, a row a step (at most [`MAX_ROWS`]) and a row of air over them, so
/// the plan does not read as the last lines of the reply streaming above
/// it. No line, no plan: the plan is drawn only while its agent works.
pub(crate) fn rows(pane: &ChatPane, cols: u16) -> u16 {
    match (
        crate::chatcodeline::code_rows(pane, cols),
        pane.checklist.len(),
    ) {
        (0, _) => 0,
        (line, 0) => line,
        (line, n) => line + n.min(MAX_ROWS) as u16 + 1,
    }
}

/// Of `space` rows over the code line, the plan's: as many as its `steps`
/// want, and what is left over is the air. A short pane takes the air back
/// before it takes a step.
pub(crate) fn plan_space(space: u16, steps: usize) -> u16 {
    space.min(steps.min(MAX_ROWS) as u16)
}

/// The step the agent is on: the active one, else the first not done, else
/// the last.
fn anchor(steps: &[Step]) -> usize {
    let at = |s: StepState| steps.iter().position(|x| x.state == s);
    at(StepState::Active)
        .or_else(|| steps.iter().position(|x| x.state != StepState::Done))
        .unwrap_or(steps.len().saturating_sub(1))
}

/// The rows `steps` take in `r` rows. All of them when they fit. Otherwise a
/// window over the step the agent is on, with one step of what came before
/// it when there is room, filling every row, and the rest counted on a head
/// and a tail row. Below three rows a count fits only beside the last step;
/// elsewhere the window is the step and what follows it.
pub(crate) fn window(steps: &[Step], r: usize) -> Vec<Row> {
    let n = steps.len();
    if n <= r {
        return (0..n).map(Row::Step).collect();
    }
    let at = anchor(steps);
    // (head, count, tail) for a window starting at `s`, when it shows `at`.
    let fit = |s: usize| -> Option<(usize, usize, usize)> {
        let head = usize::from(s > 0);
        let room = r.checked_sub(head)?;
        let (count, tail) = match s + room >= n {
            true => (n - s, 0),
            false => (room.checked_sub(1)?, 1),
        };
        (at >= s && at < s + count).then_some((head, count, tail))
    };
    let want = at.saturating_sub(1);
    let best = (0..=at)
        .filter_map(|s| fit(s).map(|f| (s, f)))
        .max_by_key(|&(s, (h, c, t))| (h + c + t, std::cmp::Reverse(s.abs_diff(want))));
    let Some((s, (head, count, tail))) = best else {
        let s = at.min(n - r);
        return (s..s + r).map(Row::Step).collect();
    };
    let mut out = Vec::with_capacity(r);
    if head == 1 {
        let all_done = steps[..s].iter().all(|x| x.state == StepState::Done);
        out.push(if all_done {
            Row::Done(s)
        } else {
            Row::Earlier(s)
        });
    }
    out.extend((s..s + count).map(Row::Step));
    if tail == 1 {
        out.push(Row::More(n - s - count));
    }
    out
}

/// The line a turn that ends with its plan unfinished leaves behind:
/// `○ plan 2/5 · left: Run the tests, +2 more`. The plan is drawn only while
/// its agent works, so a turn that stopped, failed or simply answered at 2/5
/// left no trace of the three steps it never did. `None` when every step is
/// done — and when the one left is the LAST, still marked active: Claude Code
/// leaves its final step in progress while it writes the answer.
pub(crate) fn left_note(steps: &[Step]) -> Option<String> {
    let left: Vec<&Step> = steps
        .iter()
        .filter(|s| s.state != StepState::Done)
        .collect();
    match left.as_slice() {
        [] => return None,
        [last] if last.state == StepState::Active && std::ptr::eq(*last, steps.last()?) => {
            return None
        }
        _ => {}
    }
    let first = clip(&left[0].text, 48);
    let more = match left.len() - 1 {
        0 => String::new(),
        n => format!(", +{n} more"),
    };
    Some(format!(
        "\u{25cb} plan {} \u{b7} left: {first}{more}",
        tally(steps)
    ))
}

fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    match s.chars().count() > max {
        true => format!("{}\u{2026}", s.chars().take(max - 1).collect::<String>()),
        false => s.to_string(),
    }
}

/// `done/total`, for the edge of the plan's first row.
pub(crate) fn tally(steps: &[Step]) -> String {
    let done = steps.iter().filter(|s| s.state == StepState::Done).count();
    format!("{done}/{}", steps.len())
}

#[cfg(test)]
#[path = "chatchecklist_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatchecklistleft_tests.rs"]
mod left_tests;
