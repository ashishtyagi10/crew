//! What the pane takes besides keys: a paste, and the wheel.
use winit::keyboard::{Key, NamedKey};

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

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
