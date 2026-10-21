//! The list under the mouse (goal 2026-10-20-inbox-list-look, item 15): a
//! click selects a thread and a second click opens it, a click on a `★`
//! takes the star off, and the thread under the pointer wears a lighter
//! wash than the selection. Beside an open thread on a wide pane (`split`),
//! a click opens the thread at once: the reader is already open.
use super::{InboxPane, Stage};

impl InboxPane {
    /// The thread drawn at `(row, col)` of the pane, on its names, its
    /// subject or anywhere across its lines.
    pub(crate) fn thread_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<usize> {
        let width = match (&self.reading, self.split(cols)) {
            (None, _) => cols,
            (Some(_), Some(s)) => s.list,
            (Some(_), None) => return None,
        };
        let body = self.body_rows(cols, rows);
        if !matches!(self.stage, Stage::Ready(_)) || col >= width || row >= body {
            return None;
        }
        let shown = self.shown(body);
        let (_, i) = shown
            .threads()
            .find(|&(r, _)| (r..r + shown.lines).contains(&row))?;
        Some(i)
    }

    /// A left click at `(row, col)`: true when it landed on a thread.
    pub(crate) fn click(&mut self, cols: u16, rows: u16, row: u16, col: u16) -> bool {
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
        let width = self.split(cols).map_or(cols, |s| s.list);
        let shown = self.shown(self.body_rows(cols, rows));
        let Some((top, _)) = shown.threads().find(|&(_, j)| j == i) else {
            return false;
        };
        let mut cells = Vec::new();
        super::row::washed(&mut cells, th, top, None, &self.ctx(width, &shown));
        cells
            .iter()
            .any(|c| c.row == row && c.col == col && c.c == '\u{2605}')
    }
}

#[cfg(test)]
#[path = "mouse_tests.rs"]
mod tests;
