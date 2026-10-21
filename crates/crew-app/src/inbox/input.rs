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
    /// The wheel moves the selection in the list and scrolls the reader.
    pub fn scroll_wheel(&mut self, lines: i32, cols: u16, rows: u16) {
        let (up, n) = (lines > 0, lines.unsigned_abs() as usize);
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
