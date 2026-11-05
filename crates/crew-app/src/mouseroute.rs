//! Where a click in a terminal pane goes when the program asked for the mouse
//! (see [`crate::mousereport`]). The press is checked AFTER crew's own targets
//! (toasts, gutter, docked buttons, the legend row), which sit outside a
//! terminal's content cells or are tested first in `events`, so they still win.
use std::io::Write;

use winit::event::MouseButton;

use crate::app::CrewApp;
use crate::mousereport::{button_code, mod_bits, motion_code, report, reports_click};
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

/// What the routing remembers between events: the press waiting for its
/// release, and the pane and cell the pointer was last over, so a move is
/// reported once per cell crossed rather than once per pixel.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MouseState {
    held: Option<Held>,
    last: Option<(usize, (u16, u16))>,
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
        self.mouse.held = Some(Held {
            pane: i,
            code,
            cell: (col, row),
        });
        self.mouse.last = Some((i, (col, row)));
        true
    }

    /// Send the release for a press [`Self::report_press`] reported. True
    /// when `b` ends such a press, so crew's own release (selection copy,
    /// fold toggle, card drop) is skipped for a gesture that was never crew's.
    pub(crate) fn report_release(&mut self, b: MouseButton) -> bool {
        let Some(held) = self.mouse.held else {
            return false;
        };
        if button_code(b) != Some(held.code & 3) {
            return false;
        }
        self.mouse.held = None;
        // Off the pane, the release lands where the drag last was reported.
        let cell = match (self.term_content_cell(), self.mouse.last) {
            (Some((i, col, row)), _) if i == held.pane => (col, row),
            (_, Some((i, cell))) if i == held.pane => cell,
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

    /// Report a pointer move to the program under it, once per cell crossed:
    /// a drag (button held, DECSET 1002 or 1003) to the pane the press went
    /// to, a bare move (1003) to whichever pane it is over. Nothing while
    /// crew's own selection drag is in hand, or with Shift/Cmd held and no
    /// reported button down. True when bytes were written.
    pub(crate) fn report_motion(&mut self) -> bool {
        let here = self.term_content_cell();
        let key = here.map(|(i, col, row)| (i, (col, row)));
        if key == self.mouse.last {
            return false;
        }
        self.mouse.last = key;
        let Some((i, col, row)) = here else {
            return false;
        };
        let held = self.mouse.held;
        let mods = self.mods.state();
        let free = held.is_none() && (self.drag.is_some() || mods.shift_key());
        if free || mods.super_key() || held.is_some_and(|h| h.pane != i) {
            return false;
        }
        let Some(PaneContent::Terminal(t)) = self.panes.get_mut(i).map(|p| &mut p.content) else {
            return false;
        };
        let m = t.pty.input_modes();
        let bits = mod_bits(mods.alt_key(), mods.control_key());
        let Some(code) = motion_code(&m, held.map(|h| h.code), bits) else {
            return false;
        };
        // Written straight to the program, without the snap to the live
        // bottom a click makes: hovering must not scroll the view.
        let bytes = report(code, (col, row), m.sgr_mouse, true);
        let _ = t.input.write_all(&bytes).and_then(|_| t.input.flush());
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
