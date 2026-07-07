//! Scrollback routing for panes (mouse wheel and Shift+PageUp/Down).
use std::io::Write;

use winit::event::MouseScrollDelta;

use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};

/// Pixels per scroll line for trackpad/pixel-precise wheel deltas. Roughly one
/// text row; tuned so the scroll speed matches a traditional wheel notch.
const PIXELS_PER_LINE: f32 = 24.0;

/// Write `bytes` to a terminal pane's program, snapping to the live bottom first.
fn forward_to_program(pane: &mut Pane, bytes: &[u8]) {
    if let PaneContent::Terminal(t) = &mut pane.content {
        t.pty.scroll_to_bottom();
        let _ = t.input.write_all(bytes).and_then(|_| t.input.flush());
    }
}

/// Forward a wheel scroll to a full-screen program (alt-screen/mouse app) as
/// mouse or arrow-key bytes. Returns true when forwarded — false means the pane
/// owns no full-screen program, so the caller should scroll local scrollback.
fn forward_wheel(pane: &mut Pane, lines: i32, cell: (u16, u16)) -> bool {
    let PaneContent::Terminal(t) = &pane.content else {
        return false;
    };
    let Some(bytes) = crate::altscroll::wheel_bytes(&t.pty.input_modes(), lines, cell) else {
        return false;
    };
    forward_to_program(pane, &bytes);
    true
}

/// Forward a page scroll to a full-screen program as a PageUp/PageDown key.
/// Returns false outside the alternate screen (use local scrollback instead).
fn forward_page(pane: &mut Pane, up: bool) -> bool {
    let PaneContent::Terminal(t) = &pane.content else {
        return false;
    };
    let Some(bytes) = crate::altscroll::page_bytes(&t.pty.input_modes(), up) else {
        return false;
    };
    forward_to_program(pane, &bytes);
    true
}

/// Scroll one pane's content by `lines` (positive = up/older). `cursor_col`
/// is the pane-relative column the cursor sits over (mouse-wheel scrolls
/// only); `None` for keyboard-triggered scrolls (Shift+PageUp/Down, Home/End)
/// — the markdown viewer falls back to its active side in that case.
fn scroll_pane(pane: &mut Pane, lines: i32, cursor_col: Option<u16>) {
    let cols = pane.grid.cols;
    let rows = pane.grid.rows;
    match &mut pane.content {
        PaneContent::Terminal(t) => t.pty.scroll(lines),
        PaneContent::Chat(c) => c.scroll(lines, pane.grid.cols, pane.grid.rows),
        PaneContent::Settings(s) => s.scroll(lines),
        PaneContent::Far(f) => f.scroll(lines),
        // The swarm view always renders the current fleet; nothing to scroll.
        PaneContent::Swarm(_) => {}
        PaneContent::Markdown(m) => m.scroll_wheel(cols, rows, cursor_col, lines),
    }
}

impl CrewApp {
    /// Convert one wheel/trackpad delta into whole scroll lines, carrying the
    /// sub-line remainder across calls. Without this, macOS trackpads — which
    /// emit a stream of small pixel deltas — had every tick rounded to zero, so
    /// slow scrolling never moved a pane at all.
    pub(crate) fn wheel_lines(&mut self, delta: MouseScrollDelta) -> i32 {
        self.scroll_accum += match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(p) => p.y as f32 / PIXELS_PER_LINE,
        };
        let lines = self.scroll_accum.trunc() as i32;
        self.scroll_accum -= lines as f32;
        lines
    }

    /// Route a mouse-wheel scroll to the pane under the cursor. A full-screen
    /// program (alt-screen/mouse app) receives the wheel as input bytes; any
    /// other pane scrolls its own scrollback.
    pub(crate) fn scroll_at_cursor(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        // A generic selection's cell coords go stale once content scrolls.
        self.cell_sel = None;
        let Some(i) = self.pane_at_cursor() else {
            return;
        };
        // The hovered cell positions forwarded mouse events; (0,0) when unknown.
        let cell = self.cursor_term_cell().map_or((0, 0), |(_, c, r)| (c, r));
        // Which pane-relative column the cursor sits over — the markdown
        // viewer uses this to scroll whichever half (source/preview) it's
        // hovering rather than always the active one.
        let cursor_col = self.cursor_any_cell().map(|(_, c, _)| c);
        if let Some(pane) = self.panes.get_mut(i) {
            if !forward_wheel(pane, lines, cell) {
                scroll_pane(pane, lines, cursor_col);
            }
            self.redraw();
        }
    }

    /// Scroll the focused pane by one page (Shift+PageUp/PageDown).
    pub(crate) fn scroll_focused_page(&mut self, up: bool) {
        if let Some(pane) = self.panes.get_mut(self.focused) {
            if !forward_page(pane, up) {
                let page = pane.grid.rows.saturating_sub(1).max(1) as i32;
                scroll_pane(pane, if up { page } else { -page }, None);
            }
            self.redraw();
        }
    }

    /// Jump the focused pane to the top / bottom of its scrollback (Shift+Home/End).
    pub(crate) fn scroll_focused_end(&mut self, to_top: bool) {
        if let Some(pane) = self.panes.get_mut(self.focused) {
            // The grid clamps a huge delta to the available history range.
            scroll_pane(pane, if to_top { i32::MAX / 2 } else { i32::MIN / 2 }, None);
            self.redraw();
        }
    }
}

#[cfg(test)]
#[path = "scroll_tests.rs"]
mod scroll_tests;
