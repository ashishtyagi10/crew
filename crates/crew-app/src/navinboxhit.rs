//! The Inbox section in the app: what is under the pointer, what a click opens.
//!
//! A click on a thread opens the inbox pane ON that thread. The pane may have just been made,
//! its list not yet back from Gmail, so the thread is asked for by id and opened the tick the
//! pane's rows hold it ([`CrewApp::tick_inbox_want`]); a list that lands without it lets it go.
use std::sync::Mutex;

use crate::app::CrewApp;
use crate::navsection::NavSection;
use crate::pane::PaneContent;

/// The thread a click asked for, by id, and until when — a list that never lands must not
/// open it minutes later, in the middle of something else.
static WANT: Mutex<Option<(String, u64)>> = Mutex::new(None);
/// How long a click waits for the inbox's list.
const WAIT_MS: u64 = 20_000;

impl CrewApp {
    /// The unread thread under the pointer, as an index into `inboxglance::unread`.
    pub(crate) fn inbox_item_at_cursor(&self) -> Option<usize> {
        let (row, rows) = self.section_row(NavSection::Inbox)?;
        let (first, n) = self.list_window(NavSection::Inbox, rows);
        crate::navinbox::item_at(row, n).map(|k| first + k)
    }

    /// A press on the Inbox panel: a thread opens in the inbox pane, any other row below the
    /// rule opens the inbox. `true` when it was the panel's.
    pub(crate) fn inbox_press(&mut self) -> bool {
        let Some((row, _)) = self.section_row(NavSection::Inbox) else {
            return false;
        };
        if row == 0 {
            return false;
        }
        self.inbox_open_item(self.inbox_item_at_cursor());
        self.redraw();
        true
    }

    /// Open unread thread `k` (an index into `inboxglance::unread`) as a click on it does;
    /// `None`: the inbox. The click and the keyboard's Enter both come here.
    pub(crate) fn inbox_open_item(&mut self, k: Option<usize>) {
        let want = k
            .and_then(|k| crate::inboxglance::unread().get(k).map(|t| t.id.clone()))
            .map(|id| (id, crate::anim::now_ms() + WAIT_MS));
        *WANT.lock().unwrap_or_else(|p| p.into_inner()) = want;
        self.inbox_command("");
        self.tick_inbox_want();
    }

    /// Open the thread a click asked for, once the inbox pane's list holds it. `true` when
    /// one opened.
    pub(crate) fn tick_inbox_want(&mut self) -> bool {
        let mut want = WANT.lock().unwrap_or_else(|p| p.into_inner());
        let Some((id, until)) = want.clone() else {
            return false;
        };
        if crate::anim::now_ms() > until {
            *want = None;
            return false;
        }
        let pane = self.panes.iter_mut().find_map(|p| match &mut p.content {
            PaneContent::Inbox(i) => Some(i),
            _ => None,
        });
        let Some(p) = pane else {
            // The pane was closed before its list came back.
            *want = None;
            return false;
        };
        if p.rows.is_empty() {
            return false;
        }
        *want = None;
        match p.rows.iter().position(|t| t.id == id) {
            Some(i) => p.open_at(i),
            None => false,
        }
    }

    /// The Inbox section's cells for a panel `cols` × `rows`.
    pub(crate) fn inbox_cells(&self, cols: u16, rows: u16) -> Vec<crew_render::CellView> {
        let win = self.list_window(NavSection::Inbox, rows);
        let mut out = crate::navinbox::cells(
            &crate::inboxglance::state(),
            &crate::inboxglance::unread(),
            win,
            self.inbox_item_at_cursor(),
            crate::chattime::unix_now_ms() as i64,
            (cols, rows),
        );
        self.list_bar(&mut out, NavSection::Inbox, win, cols);
        out
    }
}

/// The thread a click or Enter asked for and the pane has not opened yet, by id; clears it.
#[cfg(test)]
pub(crate) fn take_want() -> Option<String> {
    let mut want = WANT.lock().unwrap_or_else(|p| p.into_inner());
    want.take().map(|(id, _)| id)
}

#[cfg(test)]
#[path = "navinboxhit_tests.rs"]
mod tests;
