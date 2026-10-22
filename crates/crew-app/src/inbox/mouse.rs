//! The list under the mouse (goal 2026-10-20-inbox-list-look, item 15): a
//! click selects a thread and a second click opens it, a click on a `★`
//! takes the star off, and the thread under the pointer wears a lighter
//! wash than the selection. Beside an open thread on a wide pane (`split`),
//! a click opens the thread at once: the reader is already open.
use super::keys::InboxAction;
use super::InboxPane;

/// What a click did.
pub(crate) enum Clicked {
    Miss,
    Done,
    /// A key that asks the app to act: close the pane, open `/keys`.
    Then(InboxAction),
    /// A link or a button in the open mail, by a plain click, as in Gmail.
    Open(String),
}

impl InboxPane {
    /// The thread drawn at `(row, col)` of the pane, on its names, its
    /// subject or anywhere across its lines.
    pub(crate) fn thread_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<usize> {
        let inner = self.cards(cols, rows).list?.inner();
        let (row, _) = inner.local(row, col)?;
        let shown = self.shown(inner.h);
        let (_, i) = shown
            .threads()
            .find(|&(r, _)| (r..r + shown.lines).contains(&row))?;
        Some(i)
    }

    /// Whether a click at `(row, col)` would do something: a thread, a
    /// keycap, the counter, a folded mail, a link, the reply box's files.
    /// The tile and the mail window both make the pointer a hand there.
    pub(crate) fn clickable_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> bool {
        let w = self.reader_box(cols).1;
        let folded = self.mail_local(cols, rows, row, col);
        self.thread_at(cols, rows, row, col).is_some()
            || self.foot_key_at(cols, rows, row, col).is_some()
            || self.counter_at(cols, rows, row, col).is_some()
            || folded.is_some_and(|(r, _)| self.folded_at(w, r).is_some())
            || self.link_at(cols, rows, row, col).is_some()
            || self.compose_spot_at(cols, rows, row, col).is_some()
    }

    /// A left click at `(row, col)`: on a thread, a key in the foot, or one
    /// of the reader's `‹ ›`.
    pub(crate) fn click(&mut self, cols: u16, rows: u16, row: u16, col: u16) -> Clicked {
        if let Some(k) = self.foot_key_at(cols, rows, row, col) {
            if k == super::footkeys::SEND {
                self.send_reply();
                return Clicked::Done;
            }
            return match super::footkeys::key_of(k).map(|key| self.press(&key, cols, rows)) {
                Some(Some(InboxAction::Redraw) | None) => Clicked::Done,
                Some(Some(act)) => Clicked::Then(act),
                None => Clicked::Miss,
            };
        }
        if let Some(older) = self.counter_at(cols, rows, row, col) {
            self.note = self.next_thread(older).into();
            return Clicked::Done;
        }
        // On the reply box: the caret to the line clicked.
        if self.compose_click(cols, rows, row, col) || self.todo_click(cols, rows, row, col) {
            return Clicked::Done;
        }
        let width = self.reader_box(cols).1;
        if let Some((r, _)) = self.mail_local(cols, rows, row, col) {
            if self.unfold_at(width, r) {
                return Clicked::Done;
            }
        }
        if let Some(url) = self
            .reading
            .as_ref()
            .and(self.link_at(cols, rows, row, col))
        {
            return Clicked::Open(url);
        }
        match self.click_list(cols, rows, row, col) {
            true => Clicked::Done,
            false => Clicked::Miss,
        }
    }

    fn click_list(&mut self, cols: u16, rows: u16, row: u16, col: u16) -> bool {
        let Some(i) = self.thread_at(cols, rows, row, col) else {
            return false;
        };
        if self.star_at(cols, rows, i, row, col) {
            self.select(i);
            self.triage("s");
            return true;
        }
        match (self.reading.is_some(), i == self.sel) {
            (true, true) => {}
            (true, false) | (false, true) => {
                self.open_at(i);
            }
            (false, false) => self.select(i),
        }
        true
    }

    /// Whether `(row, col)` is thread `i`'s `★`, as its row is drawn.
    fn star_at(&self, cols: u16, rows: u16, i: usize, row: u16, col: u16) -> bool {
        let Some(th) = self.rows.get(i).filter(|t| t.starred) else {
            return false;
        };
        let Some(inner) = self.cards(cols, rows).list.map(|b| b.inner()) else {
            return false;
        };
        let Some((row, col)) = inner.local(row, col) else {
            return false;
        };
        let shown = self.shown(inner.h);
        let Some((top, _)) = shown.threads().find(|&(_, j)| j == i) else {
            return false;
        };
        let mut cells = Vec::new();
        super::row::washed(&mut cells, th, top, None, &self.ctx(inner.w, &shown));
        cells
            .iter()
            .any(|c| c.row == row && c.col == col && c.c == '\u{2605}')
    }
}

#[cfg(test)]
#[path = "mouse_tests.rs"]
mod tests;
