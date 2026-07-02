//! The crew pane's live activity row: while agents work, one chip per active
//! agent showing who handed it the work — `⠹ user ⇢ planner 4s` — so the pane
//! shows the crew's interactions as they happen, not just a busy flag.
use std::time::Instant;

use crew_render::CellView;

use crate::chatroster::agent_color;

/// One currently-thinking agent, as tracked from `Activity` events.
pub(crate) struct ActiveAgent {
    /// The agent doing the work.
    pub name: String,
    /// Who handed it the work (`"user"`, a peer agent, …; may be empty).
    pub from: String,
    /// When it started thinking — drives the spinner and elapsed label.
    pub since: Instant,
}

/// Milliseconds per spinner frame.
const FRAME_MS: u128 = 120;

/// Append `s` at `(row, col..)` in `fg`, clipped to `cols`; returns the next column.
fn push(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    cols: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    let mut x = col;
    for ch in s.chars() {
        if x >= cols {
            break;
        }
        cells.push(CellView {
            col: x,
            row,
            c: ch,
            fg,
            bg,
            bold,
            italic: false,
        });
        x += 1;
    }
    x
}

/// Build the activity row at `row`: `⠹ from ⇢ agent Ns` chips, two spaces
/// apart, clipped to the pane width. Empty when nobody is working.
pub(crate) fn activity_cells(cols: u16, row: u16, active: &[ActiveAgent]) -> Vec<CellView> {
    let mut cells = Vec::new();
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let mut x = 0u16;
    for (i, a) in active.iter().enumerate() {
        if i > 0 {
            x += 2; // gap between chips
        }
        if x >= cols {
            break;
        }
        let frame =
            ((a.since.elapsed().as_millis() / FRAME_MS) as usize) % crate::update::SPINNER.len();
        let spin = crate::update::SPINNER[frame];
        x = push(&mut cells, row, x, cols, &format!("{spin} "), accent, false);
        if !a.from.is_empty() {
            x = push(&mut cells, row, x, cols, &a.from, t.text_muted, false);
            // A dashed arrow (⇢): work flowing from the sender to the agent.
            x = push(&mut cells, row, x, cols, " \u{21e2} ", t.text_muted, false);
        }
        x = push(
            &mut cells,
            row,
            x,
            cols,
            &a.name,
            agent_color(&a.name),
            true,
        );
        let secs = format!(" {}s", a.since.elapsed().as_secs());
        x = push(&mut cells, row, x, cols, &secs, t.text_muted, false);
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(name: &str, from: &str) -> ActiveAgent {
        ActiveAgent {
            name: name.into(),
            from: from.into(),
            since: Instant::now(),
        }
    }

    fn text(cells: &[CellView], cols: usize) -> String {
        let mut line = vec![' '; cols];
        for c in cells {
            line[c.col as usize] = c.c;
        }
        line.into_iter().collect()
    }

    #[test]
    fn chip_shows_who_works_for_whom_with_elapsed() {
        let cells = activity_cells(80, 2, &[active("planner", "user")]);
        let line = text(&cells, 80);
        assert!(line.contains("user \u{21e2} planner 0s"), "got: {line}");
        assert!(cells.iter().all(|c| c.row == 2));
        // The worker's name is the bold part of the chip.
        assert!(cells.iter().any(|c| c.bold));
    }

    #[test]
    fn parallel_agents_get_one_chip_each() {
        let cells = activity_cells(80, 2, &[active("planner", "user"), active("coder", "user")]);
        let line = text(&cells, 80);
        assert!(line.contains("planner"), "got: {line}");
        assert!(line.contains("coder"), "got: {line}");
    }

    #[test]
    fn empty_from_skips_the_arrow() {
        let line = text(&activity_cells(80, 2, &[active("coder", "")]), 80);
        assert!(!line.contains('\u{21e2}'), "got: {line}");
        assert!(line.contains("coder 0s"), "got: {line}");
    }

    #[test]
    fn clips_to_width_and_empty_when_idle() {
        assert!(activity_cells(80, 2, &[]).is_empty());
        let cells = activity_cells(10, 2, &[active("a-very-long-name", "user")]);
        assert!(cells.iter().all(|c| c.col < 10));
    }
}
