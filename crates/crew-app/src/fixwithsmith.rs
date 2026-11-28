//! Fix a failed command with agent smith: the key and the click on the mark.
//!
//! `cargo test` exits 101; the answer is one gesture away, not a copy-paste
//! of the scrollback into another pane. Either gesture opens (or focuses)
//! agent smith and PREFILLS its composer with the failure
//! ([`crate::failctx`]). It never sends: a click must not spend tokens, and
//! the user may want to say more than "fix it".
use crate::app::CrewApp;
use crate::askbar::{EXPLAIN_BYTES, EXPLAIN_LINES};
use crate::failctx::{self, Failure};
use crate::pane::PaneContent;

/// The chord, in the Mac's spelling. `;` because off a Mac every letter is
/// taken (`winkeys`), and the key is not spent in the terminal.
pub(crate) const KEY: &str = "Cmd+;";

impl CrewApp {
    /// The failure of pane `idx`'s last command, when it is a terminal whose
    /// shell reported a non-zero exit. `None` for anything else, in which
    /// case the callers fall back to what they did before.
    pub(crate) fn failure_of(&mut self, idx: usize) -> Option<Failure> {
        self.failure_at(idx, None)
    }

    /// [`Self::failure_of`], or with `row` the failed command whose mark is
    /// on that visible row — an older failure than the last, clicked.
    fn failure_at(&mut self, idx: usize, row: Option<u16>) -> Option<Failure> {
        let pane = self.panes.get_mut(idx)?;
        let (cols, rows, dir) = (pane.grid.cols, pane.grid.rows, pane.dir.clone());
        let PaneContent::Terminal(t) = &mut pane.content else {
            return None;
        };
        let span = match row {
            None => failctx::last_failed(&t.spans)?,
            Some(r) => {
                crate::failmark::in_view(t, rows)
                    .into_iter()
                    .find(|f| f.0 == r)?
                    .1
            }
        };
        // Paging the scrollback is the cost; skip it for the common success.
        let span = span.clone();
        let now = t.pty.lines_seen();
        let text = crate::dump::capture_scrollback(&mut t.pty, cols, rows);
        failctx::failure_of(&span, &text, now, dir.as_deref())
    }

    /// What `??` sends for pane `idx`: its failed command's own output when
    /// the last command failed, else the tail of its screen as before.
    pub(crate) fn explain_context(&mut self, idx: usize) -> Result<String, &'static str> {
        const FOCUS: &str = "?? reads a pane \u{2014} focus a terminal first";
        if let Some(f) = self.failure_of(idx) {
            return Ok(f.explain_context());
        }
        let pane = self.panes.get_mut(idx).ok_or(FOCUS)?;
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let PaneContent::Terminal(t) = &mut pane.content else {
            return Err(FOCUS);
        };
        let text = crate::dump::capture_scrollback(&mut t.pty, cols, rows);
        let tail = crate::askbar::context_tail(&text, EXPLAIN_LINES, EXPLAIN_BYTES);
        Ok(tail)
    }

    /// Open agent smith with pane `idx`'s failure in its composer: the last
    /// command's, or with `row` the one whose mark was clicked.
    pub(crate) fn fix_with_smith(&mut self, idx: usize, row: Option<u16>) {
        let Some(failure) = self.failure_at(idx, row) else {
            self.set_status(
                "nothing failed here \u{2014} fix needs a shell that reports exit status",
            );
            return;
        };
        self.spawn_crew_pane();
        match self.prefill_smith(&failure.prefill()) {
            true => self.set_status("agent smith has the failure \u{2014} edit it, then Enter"),
            false => self.set_status_err("could not open agent smith"),
        }
    }

    /// Put `draft` in the composer of the open agent smith pane, under any
    /// draft already there rather than over it. Never sends. `false` when
    /// there is no smith pane to put it in.
    pub(crate) fn prefill_smith(&mut self, draft: &str) -> bool {
        let smith = self
            .panes
            .iter_mut()
            .find(|p| p.label.as_deref() == Some("crew"));
        let Some(PaneContent::Chat(c)) = smith.map(|p| &mut p.content) else {
            return false;
        };
        match c.input.is_empty() {
            true => c.paste_composer(draft),
            false => c.paste_composer(&format!("\n{draft}")),
        }
        true
    }

    /// The chord: fix the focused pane's failure.
    pub(crate) fn fix_focused_with_smith(&mut self) {
        self.fix_with_smith(self.focused, None);
    }

    /// The pane and visible row of the failed-command mark under the
    /// pointer (`failmark`).
    pub(crate) fn failed_mark_at_cursor(&self) -> Option<(usize, u16)> {
        if !crate::bordermarks::on() {
            return None;
        }
        let (cw, ch, ..) = self.frame_geometry()?;
        let on_mark = |i: usize, rect| {
            let p = self.panes.get(i).filter(|p| !p.hidden)?;
            let PaneContent::Terminal(t) = &p.content else {
                return None;
            };
            let rows = crate::failmark::rows(t, p.grid.rows);
            Some((i, crate::failmark::hit(rect, (cw, ch), self.cursor, &rows)?))
        };
        self.pane_hit_rects()
            .into_iter()
            .find_map(|(i, rect)| on_mark(i, rect))
    }

    /// A left press on a failed command's mark opens smith with THAT command.
    /// `true` when the press was the mark's, so nothing else sees it.
    pub(crate) fn fix_mark_press(&mut self) -> bool {
        let Some((i, row)) = self.failed_mark_at_cursor() else {
            return false;
        };
        self.focused = i;
        self.fix_with_smith(i, Some(row));
        true
    }
}

// Both spawn `/bin/sh` for a real pty: Unix only (Linux CI runs them).
#[cfg(all(test, unix))]
#[path = "fixwithsmith_tests.rs"]
mod tests;

#[cfg(all(test, unix))]
#[path = "fixclick_tests.rs"]
mod click_tests;
