//! The composer's caret and the editing a terminal agent's prompt has: ←/→ a
//! char, Alt+←/→ a word, Home/End and Ctrl+A/E the line's ends, Delete, Ctrl+K
//! and Ctrl+U to the line's end and start, Ctrl+W / Alt+Backspace the word
//! before it; → at the very end still takes the suggestion. Stored as
//! `ChatPane::caret_back` — chars AFTER the caret, 0 at the end — so every
//! writer that knows nothing of a caret (an append, a recalled line, Tab, a
//! pick) leaves it at the end; `on_typed` resets it when one replaced the draft.
use crate::chat::ChatPane;
use crate::chatkeys::{ChatAction, ChatInput};

/// What a caret key does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Left,
    Right,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    /// The char after the caret.
    Delete,
    /// The word before the caret (Ctrl+W, Alt+Backspace).
    WordBack,
    /// To the line's end (Ctrl+K; at the end, the break) / start (Ctrl+U).
    KillEnd,
    KillStart,
}

/// Pure input reducer with the caret at the end (the input bar's): `enter`
/// takes the whole input, `backspace` pops, `ch` (non-control, or `\n`) pushes.
pub fn input_reduce(
    input: &mut String,
    ch: Option<char>,
    enter: bool,
    backspace: bool,
) -> Option<String> {
    reduce_at(input, 0, ch, enter, backspace)
}

/// [`input_reduce`] with the caret `back` chars before the end: a char lands
/// at the caret, Backspace takes the one before it.
pub(crate) fn reduce_at(
    input: &mut String,
    back: usize,
    ch: Option<char>,
    enter: bool,
    backspace: bool,
) -> Option<String> {
    if enter {
        return Some(std::mem::take(input));
    }
    let mut chars: Vec<char> = input.chars().collect();
    let at = chars.len() - back.min(chars.len());
    match ch {
        _ if backspace && at > 0 => {
            chars.remove(at - 1);
        }
        Some(c) if !backspace && (!c.is_control() || c == '\n') => chars.insert(at, c),
        _ => return None,
    }
    *input = chars.into_iter().collect();
    None
}

/// The caret as a char index from the start of `input`.
pub(crate) fn caret_at(input: &str, back: usize) -> usize {
    let n = input.chars().count();
    n - back.min(n)
}

/// Apply `op` with the caret at `at`; the caret's new index.
pub(crate) fn apply(chars: &mut Vec<char>, at: usize, op: Op) -> usize {
    let len = chars.len();
    let at = at.min(len);
    let start = chars[..at]
        .iter()
        .rposition(|&c| c == '\n')
        .map_or(0, |i| i + 1);
    let end = chars[at..]
        .iter()
        .position(|&c| c == '\n')
        .map_or(len, |i| at + i);
    match op {
        Op::Left => at.saturating_sub(1),
        Op::Right => (at + 1).min(len),
        Op::WordLeft => word_left(chars, at),
        Op::WordRight => word_right(chars, at),
        Op::LineStart => start,
        Op::LineEnd => end,
        Op::Delete => {
            if at < len {
                chars.remove(at);
            }
            at
        }
        Op::WordBack => {
            let to = word_left(chars, at);
            chars.drain(to..at);
            to
        }
        Op::KillEnd => {
            let to = if end == at { (at + 1).min(len) } else { end };
            chars.drain(at..to);
            at
        }
        Op::KillStart => {
            chars.drain(start..at);
            start
        }
    }
}

/// The start of the word before `at` (whitespace first, then the word).
fn word_left(chars: &[char], mut at: usize) -> usize {
    while at > 0 && chars[at - 1].is_whitespace() {
        at -= 1;
    }
    while at > 0 && !chars[at - 1].is_whitespace() {
        at -= 1;
    }
    at
}

/// The end of the word after `at`.
fn word_right(chars: &[char], mut at: usize) -> usize {
    while at < chars.len() && chars[at].is_whitespace() {
        at += 1;
    }
    while at < chars.len() && !chars[at].is_whitespace() {
        at += 1;
    }
    at
}

impl ChatPane {
    /// The caret keys — and → / Tab, which only take a suggestion or complete
    /// at the very end. `Some` when consumed.
    pub(crate) fn caret_key(
        &mut self,
        k: &ChatInput,
        cwd: &std::path::Path,
    ) -> Option<Option<ChatAction>> {
        let op = match k {
            ChatInput::Caret(op) => *op,
            ChatInput::Accept if self.caret_back > 0 => Op::Right,
            ChatInput::Complete if self.caret_back > 0 => return Some(None),
            _ => return None,
        };
        let mut chars: Vec<char> = self.input.chars().collect();
        let before = chars.len();
        let at = before - self.caret_back.min(before);
        let to = apply(&mut chars, at, op);
        let edited = chars.len() != before;
        if edited {
            self.input = chars.iter().collect();
            self.history.edited(); // the text is the user's own now
        }
        self.caret_back = chars.len() - to;
        self.follow_caret(edited, cwd);
        Some(None)
    }

    /// The popups follow the token typed at the end: a caret anywhere else
    /// closes them, and an edit at the end re-syncs them as typing does.
    pub(crate) fn follow_caret(&mut self, edited: bool, cwd: &std::path::Path) {
        if self.caret_back > 0 {
            (self.palette, self.mention) = (None, None);
        } else if edited {
            let agents = self.agents.clone();
            crate::chatmention::after_edit(&mut self.mention, &self.input, || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
            self.sync_palette(cwd);
        }
    }

    /// The char just before the caret, if there is one.
    pub(crate) fn before_caret(&self) -> Option<char> {
        let at = caret_at(&self.input, self.caret_back);
        at.checked_sub(1).and_then(|i| self.input.chars().nth(i))
    }

    /// Splice `text` in at the caret (a paste, a dropped file): the chars
    /// after the caret stay after it, so `caret_back` holds.
    pub(crate) fn insert_at_caret(&mut self, text: &str) {
        let at = caret_at(&self.input, self.caret_back);
        let byte = self
            .input
            .char_indices()
            .nth(at)
            .map_or(self.input.len(), |(b, _)| b);
        self.input.insert_str(byte, text);
    }
}

#[cfg(test)]
#[path = "chatcursor_tests.rs"]
mod tests;
