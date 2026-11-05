//! Where a click in a terminal pane goes when the program asked for the mouse
//! (see [`crate::mousereport`]). The press is checked AFTER crew's own targets
//! (toasts, gutter, docked buttons, the legend row), which sit outside a
//! terminal's content cells or are tested first in `events`, so they still win.
use winit::event::MouseButton;

use crate::app::CrewApp;
use crate::mousereport::{button_code, mod_bits, report, reports_click};
use crate::pane::PaneContent;

/// A press that went to a program, waiting for its release: the pane, the
/// button code it was sent with, and the cell it landed on (the release goes
/// there if the pointer has left the pane by then).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Held {
    pane: usize,
    code: u8,
    cell: (u16, u16),
}

impl CrewApp {
    /// Report a press of `b` to the program under the cursor, focusing its
    /// pane. False (nothing written) when the cursor is not over a terminal's
    /// content, the program has no mouse mode on, or Shift/Cmd is held — the
    /// caller then runs crew's own click (selection, paste, focus).
    pub(crate) fn report_press(&mut self, b: MouseButton) -> bool {
        let Some(code) = button_code(b) else {
            return false;
        };
        let mods = self.mods.state();
        if mods.super_key() {
            return false;
        }
        let Some((i, col, row)) = self.term_content_cell() else {
            return false;
        };
        let Some(PaneContent::Terminal(t)) = self.panes.get(i).map(|p| &p.content) else {
            return false;
        };
        let m = t.pty.input_modes();
        if !reports_click(&m, mods.shift_key()) {
            return false;
        }
        let code = code | mod_bits(mods.alt_key(), mods.control_key());
        self.focus_at_cursor();
        self.drag = None;
        crate::scroll::forward_to_program(
            &mut self.panes[i],
            &report(code, (col, row), m.sgr_mouse, true),
        );
        self.mouse_held = Some(Held {
            pane: i,
            code,
            cell: (col, row),
        });
        true
    }

    /// Send the release for a press [`Self::report_press`] reported. True
    /// when `b` ends such a press, so crew's own release (selection copy,
    /// fold toggle, card drop) is skipped for a gesture that was never crew's.
    pub(crate) fn report_release(&mut self, b: MouseButton) -> bool {
        let Some(held) = self.mouse_held else {
            return false;
        };
        if button_code(b) != Some(held.code & 3) {
            return false;
        }
        self.mouse_held = None;
        let cell = match self.term_content_cell() {
            Some((i, col, row)) if i == held.pane => (col, row),
            _ => held.cell,
        };
        if let Some(p) = self.panes.get_mut(held.pane) {
            if let PaneContent::Terminal(t) = &p.content {
                // A program that turned the mouse off between press and
                // release gets nothing: it no longer reads reports.
                let m = t.pty.input_modes();
                if m.mouse {
                    let bytes = report(held.code, cell, m.sgr_mouse, false);
                    crate::scroll::forward_to_program(p, &bytes);
                }
            }
        }
        true
    }

    /// The terminal pane and cell under the cursor, only inside its grid: a
    /// cursor on the right border or the gutter is not a cell of the program.
    fn term_content_cell(&self) -> Option<(usize, u16, u16)> {
        let (i, col, row) = self.cursor_term_cell()?;
        let g = &self.panes.get(i)?.grid;
        (col < g.cols && row < g.rows).then_some((i, col, row))
    }
}

#[cfg(all(test, unix))]
#[path = "mouseroute_tests.rs"]
mod tests;
