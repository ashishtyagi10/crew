//! The click *run*: what a left press means when it lands soon after the last
//! one on the same pane. Single selects nothing (a drag is armed instead),
//! double takes the word under the cursor, triple the whole line — the gesture
//! every terminal has had for thirty years and crew did not.
//!
//! Split from [`crate::select`], which owns the drag half of the mouse.
use std::time::{Duration, Instant};

use crate::app::CrewApp;
use crate::gridsel::CellSel;
use crate::pane::PaneContent;

/// Max gap between two left clicks on the same pane for the second to
/// continue the run rather than start a new one.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// The longest run a click can reach: single, word, line. A fourth click
/// starts over at single, so a hand resting on the button cycles the gestures
/// instead of sticking on the widest one.
const MAX_RUN: u8 = 3;

/// The run length this press reaches, given the previous one. A press on a
/// different pane, or one that came too late, starts a fresh run.
fn next_run(last: Option<(Instant, usize, u8)>, now: Instant, pane: usize) -> u8 {
    match last {
        Some((t, p, n)) if p == pane && now.duration_since(t) < DOUBLE_CLICK && n < MAX_RUN => {
            n + 1
        }
        _ => 1,
    }
}

impl CrewApp {
    /// What a left press means when it lands on pane `i`, given how recently
    /// the last one did: the click *run*.
    ///
    /// Inside a pane's content, the run is the gesture every terminal has:
    /// once selects nothing (a drag is armed instead), twice selects the word
    /// under the cursor, three times the whole line — and each selection is
    /// copied, the same "select is copy" rule a drag-release follows.
    ///
    /// On a card's **border** — where there is no text to select — a double
    /// click toggles zoom, which is where the gesture moved to when the
    /// content took over the double click. It is also where the convention
    /// puts it: a window's title bar, not its contents.
    ///
    /// A press that armed a fold toggle (`fold_armed`, see `chatfold`) breaks
    /// the run in both directions — folding a card twice must not select or
    /// zoom anything.
    pub(crate) fn click_gesture(&mut self, i: usize, fold_armed: bool) {
        let now = Instant::now();
        if fold_armed {
            self.last_click = None;
            return;
        }
        let run = next_run(self.last_click, now, i);
        self.last_click = Some((now, i, run));
        // `selection_press` arms a drag exactly when the press found a content
        // cell, so this is also the answer to "was that the border?".
        let Some(cell) = self.drag.map(|d| d.anchor) else {
            if run == 2 {
                self.zoomed = !self.zoomed;
            }
            return;
        };
        match run {
            2 => self.select_run(i, cell, false),
            3 => self.select_run(i, cell, true),
            _ => {}
        }
    }

    /// Widen the selection on pane `i` around `cell` to a word (or the whole
    /// `line`) and copy it. Terminals answer through alacritty's own semantic
    /// and line selections; every other pane kind through [`crate::gridsel`],
    /// so the gesture means the same thing on a transcript as on a shell.
    fn select_run(&mut self, i: usize, (col, row): (u16, u16), line: bool) {
        // The widened selection replaces the armed drag: a hand that jitters
        // one cell after a double click must not wipe the word it just took.
        self.drag = None;
        let Some(pane) = self.panes.get_mut(i) else {
            return;
        };
        if let PaneContent::Terminal(t) = &mut pane.content {
            if line {
                t.pty.sel_line(col, row);
            } else {
                t.pty.sel_word(col, row);
            }
        } else {
            let cells = pane.cells(false);
            let span = if line {
                crate::gridsel::line_span(&cells, row)
            } else {
                crate::gridsel::word_span(&cells, col, row)
            };
            let Some((lo, hi)) = span else {
                self.cell_sel = None;
                return;
            };
            self.cell_sel = Some(CellSel {
                pane: i,
                anchor: (lo, row),
                cursor: (hi, row),
            });
        }
        if let Some(text) = self.pane_selection_text(i) {
            self.copy_text(text);
        }
    }
}

#[cfg(test)]
#[path = "selrun_tests.rs"]
mod tests;
