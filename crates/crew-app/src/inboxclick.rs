//! The app's half of the inbox list under the mouse (`inbox::mouse`): which
//! pane and cell the pointer is on, handed to the pane, which says what a
//! click there does and which thread to wash for the hover.
use crate::app::CrewApp;
use crate::inbox::keys::InboxAction;
use crate::inbox::mouse::Clicked;
use crate::pane::PaneContent;

impl CrewApp {
    /// The inbox pane under the pointer, and the cell in it.
    fn inbox_cell_at_cursor(&self) -> Option<(usize, u16, u16)> {
        let i = self.pane_at_cursor()?;
        if !matches!(self.panes[i].content, PaneContent::Inbox(_)) {
            return None;
        }
        let (row, col) = self.cursor_rowcol(i)?;
        Some((i, u16::try_from(row).ok()?, u16::try_from(col).ok()?))
    }

    /// A left click on a thread in an inbox pane: it selects, or opens, or
    /// takes the star off, and the pane takes focus. Anywhere else falls
    /// through to the focus and drag paths.
    pub(crate) fn inbox_click_at_cursor(&mut self) -> bool {
        let Some((i, row, col)) = self.inbox_cell_at_cursor() else {
            return false;
        };
        let grid = self.panes[i].grid;
        let PaneContent::Inbox(p) = &mut self.panes[i].content else {
            return false;
        };
        let then = match p.click(grid.cols, grid.rows, row, col) {
            Clicked::Miss => return false,
            Clicked::Done => None,
            Clicked::Then(act) => Some(act),
            Clicked::Open(url) => {
                self.follow_inbox_link(&url);
                None
            }
        };
        self.focused = i;
        self.input.focused = false;
        match then {
            Some(InboxAction::Close) => drop(self.close_pane(i)),
            Some(InboxAction::Keys) => self.open_help(),
            Some(InboxAction::Redraw) | None => {}
        }
        true
    }

    /// The thread under the pointer wears the hover's wash, in the one
    /// inbox pane it is over; every other loses it. `true` when one moved.
    pub(crate) fn inbox_hover_sync(&mut self) -> bool {
        let at = self.inbox_cell_at_cursor();
        let mut moved = false;
        for (i, pane) in self.panes.iter_mut().enumerate() {
            let PaneContent::Inbox(p) = &mut pane.content else {
                continue;
            };
            let grid = pane.grid;
            let hover = at
                .filter(|&(j, ..)| j == i)
                .and_then(|(_, row, col)| p.thread_at(grid.cols, grid.rows, row, col));
            moved |= std::mem::replace(&mut p.hover, hover) != hover;
        }
        moved
    }

    /// Whether pane `i` has a thread under the pointer: the hand.
    pub(crate) fn inbox_hover_on(&self, i: usize) -> bool {
        self.inbox_cell_at_cursor().is_some_and(|(at, row, col)| {
            let grid = self.panes[at].grid;
            let on = |p: &crate::inbox::InboxPane| {
                let (rx, w) = p.reader_box(grid.cols);
                p.thread_at(grid.cols, grid.rows, row, col).is_some()
                    || p.foot_key_at(grid.cols, grid.rows, row, col).is_some()
                    || col
                        .checked_sub(rx)
                        .and_then(|c| p.counter_at(w, row, c))
                        .is_some()
                    || (col >= rx && p.folded_at(w, row).is_some())
                    || p.link_at(grid.cols, grid.rows, row, col).is_some()
            };
            at == i && matches!(&self.panes[at].content, PaneContent::Inbox(p) if on(p))
        })
    }
}
