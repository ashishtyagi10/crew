//! Key handling and line editing for the input bar.
use winit::keyboard::{Key, NamedKey};

use crate::inputbar::InputBar;

impl InputBar {
    /// Handle a winit key event. `ctrl` enables readline-style line editing.
    /// Returns `Some(submitted_line)` when Enter is pressed, else `None`.
    pub fn on_key(&mut self, key: &winit::event::KeyEvent, ctrl: bool) -> Option<String> {
        if !key.state.is_pressed() {
            return None;
        }
        // Ctrl+W delete the last word, Ctrl+U clear the line.
        if ctrl {
            if let Key::Character(s) = &key.logical_key {
                match s.as_str() {
                    "w" => {
                        delete_last_word(&mut self.text);
                        return self.after_edit();
                    }
                    "u" => {
                        self.text.clear();
                        return self.after_edit();
                    }
                    _ => {}
                }
            }
        }

        let menu = crate::suggest::menu_items(&self.text);
        let menu_open = self.focused && !menu.is_empty();

        // Command-palette navigation (Up/Down) when it's open.
        if menu_open {
            match &key.logical_key {
                Key::Named(NamedKey::ArrowDown) => {
                    self.menu_sel = (self.menu_sel + 1) % menu.len();
                    return None;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.menu_sel = (self.menu_sel + menu.len() - 1) % menu.len();
                    return None;
                }
                _ => {}
            }
        } else {
            // Up/Down recall submitted history when the palette isn't open.
            match &key.logical_key {
                Key::Named(NamedKey::ArrowUp) => {
                    self.history_prev();
                    return None;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.history_next();
                    return None;
                }
                _ => {}
            }
        }

        // Tab / Right accept: fill the highlighted row (a command name, a
        // "/cmd " that opens its value picker, or a picked value), else the ghost.
        if matches!(
            &key.logical_key,
            Key::Named(NamedKey::Tab) | Key::Named(NamedKey::ArrowRight)
        ) {
            if menu_open {
                self.text = menu[self.menu_sel.min(menu.len() - 1)].fill.clone();
            } else if let Some(g) = self.ghost() {
                // Accept exactly what's shown as ghost text (history, cd, or path).
                self.text.push_str(&g);
            }
            self.menu_sel = 0;
            return None;
        }

        // Enter on an open palette: run the highlighted row when it's runnable
        // (a command or a picked value), or expand a value-picker command into
        // its list — filling "/cmd " and keeping the palette open to choose.
        if menu_open && matches!(&key.logical_key, Key::Named(NamedKey::Enter)) {
            let item = &menu[self.menu_sel.min(menu.len() - 1)];
            let fill = item.fill.clone();
            self.menu_sel = 0;
            if !item.submit {
                self.text = fill;
                return None;
            }
            self.history.push(fill.clone());
            self.text.clear();
            return Some(fill);
        }

        let (ch, enter, backspace) = match &key.logical_key {
            Key::Named(NamedKey::Enter) => (None, true, false),
            Key::Named(NamedKey::Backspace) => (None, false, true),
            Key::Named(NamedKey::Space) => (Some(' '), false, false),
            Key::Character(s) => (s.chars().next(), false, false),
            _ => (None, false, false),
        };
        let result = crate::chatinput::input_reduce(&mut self.text, ch, enter, backspace);
        self.menu_sel = 0;
        self.hist_pos = None;
        if let Some(line) = &result {
            if !line.is_empty() {
                self.history.push(line.clone());
            }
        }
        result
    }

    /// Reset transient state after a direct edit (Ctrl+W/U).
    fn after_edit(&mut self) -> Option<String> {
        self.menu_sel = 0;
        self.hist_pos = None;
        None
    }
}

/// Delete the trailing word (and any trailing whitespace) from `text`.
fn delete_last_word(text: &mut String) {
    let end = text.trim_end().len();
    let kept = text[..end]
        .rfind(char::is_whitespace)
        .map(|i| i + 1)
        .unwrap_or(0);
    text.truncate(kept);
}

#[cfg(test)]
mod tests {
    use super::delete_last_word;

    #[test]
    fn delete_last_word_cases() {
        let mut s = "ls -la foo".to_string();
        delete_last_word(&mut s);
        assert_eq!(s, "ls -la ");
        let mut s = "one".to_string();
        delete_last_word(&mut s);
        assert_eq!(s, "");
        let mut s = "trailing   ".to_string();
        delete_last_word(&mut s);
        assert_eq!(s, "");
    }
}
