//! What the pane takes besides keys: a paste, and the wheel.
use winit::keyboard::{Key, NamedKey};

use super::keys::InboxAction;
use super::{InboxPane, Stage};

impl InboxPane {
    /// A pasted client (the console's JSON) is the one to sign in with.
    pub fn paste(&mut self, text: &str) {
        match (&mut self.stage, super::grant::client_from_json(text)) {
            (Stage::Setup { found }, Some(c)) => *found = Some(("pasted client".into(), c)),
            (_, _) if self.compose.is_some() => self.type_in(text),
            (_, _) if self.typing.is_some() => {
                self.typing.get_or_insert_default().push_str(text.trim())
            }
            _ => {}
        }
    }
}

impl InboxPane {
    /// The wheel moves the selection in the list and scrolls the open mail,
    /// a reply being written or not: it is not a key, so it never reaches
    /// the reply box (where an arrow moves the caret to the `to` line).
    pub fn scroll_wheel(&mut self, lines: i32, cols: u16, rows: u16) {
        let (up, n) = (lines > 0, lines.unsigned_abs() as usize);
        let page = usize::from(self.reader_rows(cols, rows)).saturating_sub(1);
        let width = self.reader_box(cols).1;
        if let Some(r) = self.reading.as_mut() {
            let last = super::reader::height(r, width).saturating_sub(page.max(1));
            r.scroll = match up {
                true => r.scroll.saturating_sub(n),
                false => (r.scroll + n).min(last),
            };
            return;
        }
        let key = Key::Named(if up {
            NamedKey::ArrowUp
        } else {
            NamedKey::ArrowDown
        });
        for _ in 0..n.min(10) {
            self.press(&key, cols, rows);
        }
    }
}

impl InboxPane {
    /// One key, pressed: the pane's whole keyboard, testable without winit.
    /// When the reply box takes rows from the mail (opening, or growing with
    /// the words), the mail's end stays in view if it was: the newest mail,
    /// the one being answered, does not slide under the box. And a mail made
    /// shorter (`v`, `q`, `x`) is never left scrolled past its end, a card
    /// of nothing.
    pub(crate) fn press(&mut self, key: &Key, cols: u16, rows: u16) -> Option<InboxAction> {
        let (h, end) = (self.compose_h(cols, rows), self.end_in_view(cols, rows));
        let out = self.press_key(key, cols, rows);
        let (room, width) = (self.reader_rows(cols, rows), self.reader_box(cols).1);
        let grew = self.compose_h(cols, rows) > h;
        if let Some(r) = self.reading.as_mut() {
            let height = super::reader::height(r, width);
            if end && grew {
                r.scroll = height.saturating_sub(usize::from(room));
            }
            // As far as the arrows go: the last page, a row of it to spare.
            let page = usize::from(room).saturating_sub(1).max(1);
            r.scroll = r.scroll.min(height.saturating_sub(page));
        }
        out
    }

    /// Whether the open mail's last line is on screen.
    fn end_in_view(&self, cols: u16, rows: u16) -> bool {
        let (room, width) = (self.reader_rows(cols, rows), self.reader_box(cols).1);
        self.reading
            .as_ref()
            .is_some_and(|r| r.scroll + usize::from(room) >= super::reader::height(r, width))
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
