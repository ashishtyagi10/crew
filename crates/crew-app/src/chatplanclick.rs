//! The plan buttons' plumbing: which button a click lands on, the press that
//! arms it, the release that fires it, the hover that lights it — and the
//! ONE send both the buttons and the composer's Enter/Esc go through, so the
//! broker's deterministic plan gate sees the same bare word either way.
//!
//! Geometry comes from [`crate::chatplanbtn::buttons`] at the row
//! `chatplace::grants` budgeted, so a hit can never resolve against a row
//! the frame did not draw. The pure half lives in `chatplanbtn`.
use crate::app::CrewApp;
use crate::chat::ChatPane;
use crate::chatplanbtn::{buttons, Btn, PRESS_MS};
use crate::pane::PaneContent;

impl ChatPane {
    /// Answer the pending plan: `approve` runs it, `reject` discards it. The
    /// bare word, not a slash command — the broker's plan gate matches it
    /// before any model call. Returns the word sent, for the callers that
    /// report it. A no-op (`None`) when no plan is pending.
    pub(crate) fn answer_plan(&mut self, approve: bool) -> Option<&'static str> {
        if !self.plan_pending {
            return None;
        }
        self.plan_pending = false;
        self.hover_btn = None;
        self.press_btn = None;
        let word = if approve { "approve" } else { "reject" };
        self.submit_command(word.to_string());
        Some(word)
    }

    /// The absolute row the button row occupies on a `cols` × `rows` pane,
    /// or `None` when it is not drawn (no plan, or no room).
    pub(crate) fn plan_row(&self, cols: u16, rows: u16) -> Option<u16> {
        let g = crate::chatplace::grants(self, cols, rows);
        (g.plan > 0).then(|| rows - g.bottom - g.plan)
    }

    /// The button under pane cell `(row, col)`, if any.
    pub(crate) fn plan_btn_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<Btn> {
        if self.plan_row(cols, rows)? != row {
            return None;
        }
        let laid = buttons(cols, crate::glyphs::on(), None, None)?;
        laid.spans
            .into_iter()
            .find(|(_, span)| span.contains(&col))
            .map(|(b, _)| b)
    }

    /// Press at `(row, col)`: arm the button there and start its invert
    /// flash. `true` when a button was hit.
    pub(crate) fn plan_press_at(&mut self, cols: u16, rows: u16, row: u16, col: u16) -> bool {
        let Some(btn) = self.plan_btn_at(cols, rows, row, col) else {
            return false;
        };
        let now = crate::anim::now_ms();
        let flash = crate::ease::Timeline::start(now, PRESS_MS, crate::motion::level());
        self.press_btn = Some((btn, flash));
        true
    }

    /// Release at `at` (the cell under the pointer, `None` when it left the
    /// pane): fires the armed button when the gesture stayed a click on it —
    /// not a drag, not a release over the other button or off the row. The
    /// word sent, or `None` when nothing fired.
    pub(crate) fn plan_release_at(
        &mut self,
        cols: u16,
        rows: u16,
        at: Option<(u16, u16)>,
        dragged: bool,
    ) -> Option<&'static str> {
        let (armed, _) = self.press_btn.take()?;
        let under = at.and_then(|(row, col)| self.plan_btn_at(cols, rows, row, col));
        if dragged || under != Some(armed) {
            return None;
        }
        self.answer_plan(armed == Btn::Run)
    }

    /// Publish the button under `at`. `true` when the hovered button changed
    /// — the repaint signal, so a move that changed nothing costs no frame.
    pub(crate) fn plan_hover_at(&mut self, cols: u16, rows: u16, at: Option<(u16, u16)>) -> bool {
        let hit = at.and_then(|(row, col)| self.plan_btn_at(cols, rows, row, col));
        let changed = hit != self.hover_btn;
        self.hover_btn = hit;
        changed
    }
}

impl CrewApp {
    /// The chat pane under the cursor and the cell the cursor is on, as
    /// `(pane, row, col)`; `None` off every chat pane's content.
    fn chat_cell_at_cursor(&self) -> Option<(usize, u16, u16)> {
        let i = self.pane_at_cursor()?;
        let (row, col) = self.cursor_rowcol(i)?;
        matches!(self.panes[i].content, PaneContent::Chat(_)).then_some((
            i,
            u16::try_from(row).ok()?,
            u16::try_from(col).ok()?,
        ))
    }

    /// Mouse-press arm: a press on a plan button arms it (and starts its
    /// flash) instead of arming a fold. `true` when it did — the caller keeps
    /// the press out of the double-click zoom count, as for folds.
    pub(crate) fn plan_press_at_cursor(&mut self) -> bool {
        let Some((i, row, col)) = self.chat_cell_at_cursor() else {
            return false;
        };
        let grid = self.panes[i].grid;
        let PaneContent::Chat(chat) = &mut self.panes[i].content else {
            return false;
        };
        chat.plan_press_at(grid.cols, grid.rows, row, col)
    }

    /// Mouse-release: fire whichever plan button the press armed, if the
    /// pointer is still on it and the gesture never became a drag. `true`
    /// when a verdict was sent.
    pub(crate) fn plan_release(&mut self, dragged: bool) -> bool {
        let at = self.chat_cell_at_cursor();
        let Some(i) = self
            .panes
            .iter()
            .position(|p| matches!(&p.content, PaneContent::Chat(c) if c.press_btn.is_some()))
        else {
            return false;
        };
        let grid = self.panes[i].grid;
        let PaneContent::Chat(chat) = &mut self.panes[i].content else {
            return false;
        };
        let here = at.filter(|(p, _, _)| *p == i).map(|(_, r, c)| (r, c));
        chat.plan_release_at(grid.cols, grid.rows, here, dragged)
            .is_some()
    }

    /// The one left-release hook for chat-pane clicks: the plan buttons, then
    /// the card folds. A plan press never arms a fold (`events` tries it
    /// first), so both may run unconditionally.
    pub(crate) fn click_release(&mut self, dragged: bool) {
        self.plan_release(dragged);
        self.fold_release(dragged);
    }

    /// Re-publish the hovered plan button on every chat pane. `true` when any
    /// changed — the repaint signal (`pointer::pointer_sync`).
    pub(crate) fn plan_hover_sync(&mut self) -> bool {
        let at = self.chat_cell_at_cursor();
        let mut changed = false;
        for (i, pane) in self.panes.iter_mut().enumerate() {
            let PaneContent::Chat(chat) = &mut pane.content else {
                continue;
            };
            let here = at.filter(|(p, _, _)| *p == i).map(|(_, r, c)| (r, c));
            changed |= chat.plan_hover_at(pane.grid.cols, pane.grid.rows, here);
        }
        changed
    }

    /// Whether pane `i` has a plan button under the pointer — what the pointer
    /// shape asks (the same hand a link gets).
    pub(crate) fn plan_hover_on(&self, i: usize) -> bool {
        matches!(self.panes.get(i).map(|p| &p.content), Some(PaneContent::Chat(c)) if c.hover_btn.is_some())
    }
}

#[cfg(test)]
#[path = "chatplanclick_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatplanapp_tests.rs"]
mod app_tests;
