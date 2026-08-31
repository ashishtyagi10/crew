//! Editing the todo pane's input line: typing, deleting, and moving the
//! cursor through it by character, word and line.
//!
//! Split out of [`super`] for the line cap.

use super::TodoPane;

impl TodoPane {
    pub(crate) fn type_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        self.insert_at_cursor(&c.to_string());
    }

    /// THE composer write seam: splice `text` in at the cursor and advance
    /// it past the insertion. Everything textual funnels here so the tag
    /// popup (`sync_menu`) keeps tracking mid-string edits too.
    pub(crate) fn insert_at_cursor(&mut self, text: &str) {
        let mut chars: Vec<char> = self.input.chars().collect();
        let cur = self.cursor.min(chars.len());
        let add: Vec<char> = text.chars().collect();
        let added = add.len();
        chars.splice(cur..cur, add);
        self.input = chars.into_iter().collect();
        self.cursor = cur + added;
        self.sync_menu();
    }

    /// Backspace: delete the char BEFORE the cursor.
    pub(crate) fn backspace(&mut self) {
        let mut chars: Vec<char> = self.input.chars().collect();
        let cur = self.cursor.min(chars.len());
        if cur == 0 {
            return;
        }
        chars.remove(cur - 1);
        self.input = chars.into_iter().collect();
        self.cursor = cur - 1;
        self.sync_menu();
    }

    /// Forward-Delete: delete the char AT the cursor.
    pub(crate) fn delete_forward(&mut self) {
        let mut chars: Vec<char> = self.input.chars().collect();
        let cur = self.cursor;
        if cur >= chars.len() {
            return;
        }
        chars.remove(cur);
        self.input = chars.into_iter().collect();
        self.sync_menu();
    }

    /// Move the cursor by `delta` chars, clamped to the draft.
    pub(crate) fn cursor_move(&mut self, delta: isize) {
        let len = self.input.chars().count();
        self.cursor = self.cursor.min(len).saturating_add_signed(delta).min(len);
    }

    /// `Alt+Left`/`Alt+Right`: hop to the previous word start / next word
    /// end (skip whitespace, then the word).
    pub(crate) fn cursor_word(&mut self, forward: bool) {
        let chars: Vec<char> = self.input.chars().collect();
        let mut c = self.cursor.min(chars.len());
        if forward {
            while c < chars.len() && chars[c].is_whitespace() {
                c += 1;
            }
            while c < chars.len() && !chars[c].is_whitespace() {
                c += 1;
            }
        } else {
            while c > 0 && chars[c - 1].is_whitespace() {
                c -= 1;
            }
            while c > 0 && !chars[c - 1].is_whitespace() {
                c -= 1;
            }
        }
        self.cursor = c;
    }

    /// `Ctrl+A`/`Home` and `Ctrl+E`/`End`: the draft's ends.
    pub(crate) fn cursor_ends(&mut self, end: bool) {
        self.cursor = if end { self.input.chars().count() } else { 0 };
    }

    /// Up/Down inside a wrapped draft: move the cursor one visual line,
    /// keeping the nearest column (by display width). Returns `false` when
    /// there is no line to move to — the caller's cue to hand focus to the
    /// list (Up on the first line, Down on the last).
    pub(crate) fn cursor_vertical(&mut self, up: bool, cols: u16) -> bool {
        let chars: Vec<char> = self.input.chars().collect();
        let lines = super::composer::input_lines(self, cols);
        let cur = self.cursor.min(chars.len());
        let line = lines
            .iter()
            .position(|&(_, e)| cur <= e)
            .unwrap_or(lines.len() - 1);
        let target = match (up, line) {
            (true, 0) => return false,
            (true, l) => l - 1,
            (false, l) if l + 1 >= lines.len() => return false,
            (false, l) => l + 1,
        };
        let col: usize = chars[lines[line].0..cur]
            .iter()
            .map(|&c| crate::chatwidth::char_w(c))
            .sum();
        let (ts, te) = lines[target];
        let mut x = 0;
        let mut pos = ts;
        while pos < te {
            let w = crate::chatwidth::char_w(chars[pos]);
            if x + w > col {
                break;
            }
            x += w;
            pos += 1;
        }
        self.cursor = pos;
        true
    }

    pub(crate) fn sync_menu(&mut self) {
        let items = &self.items;
        super::tagmenu::after_edit(&mut self.tagmenu, &self.input, || {
            super::tagmenu::known_tags(items)
        });
    }

    pub(crate) fn reset_input(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.editing = None;
        self.tagmenu = None;
    }

    /// Esc in the composer: drop the draft (and any edit in progress).
    pub(crate) fn cancel_edit(&mut self) {
        self.reset_input();
    }
}
