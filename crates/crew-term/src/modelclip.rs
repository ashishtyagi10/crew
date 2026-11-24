//! OSC 52 both ways: the text a program copied, and a program's request to
//! READ the clipboard (`ESC ] 52 ; c ; ? BEL`, nvim's paste provider over ssh,
//! tmux's `set-clipboard`).
//!
//! The term always parses reads (`Osc52::CopyPaste`) and this hands them up
//! unanswered: whether a program may read the clipboard is a setting, and
//! whose pane is focused, both of which the app knows and the term does not.
//! Asking at reply time means a changed setting needs nothing re-applied to
//! the panes already open.
use super::{HeadlessTerm, TermCore};
use crate::listener::ClipFormat;
use crate::PtyTerm;

/// A program's request to read the clipboard. Holding one costs nothing and
/// reads nothing; only [`ClipboardAsk::reply`] builds an answer, and only
/// the app decides whether to write it back.
#[derive(Clone)]
pub struct ClipboardAsk(ClipFormat);

impl ClipboardAsk {
    /// The answer to write to the program for clipboard `text`: base64, the
    /// selector it asked about, and the terminator its query used (BEL or ST).
    pub fn reply(&self, text: &str) -> String {
        (self.0)(text)
    }
}

impl TermCore {
    /// Take any pending OSC 52 clipboard-store text (clearing it).
    pub(crate) fn take_clipboard(&self) -> Option<String> {
        self.events.clipboard.lock().unwrap().take()
    }

    pub(crate) fn take_clipboard_ask(&self) -> Option<ClipboardAsk> {
        self.events
            .clip_ask
            .lock()
            .unwrap()
            .take()
            .map(ClipboardAsk)
    }
}

impl PtyTerm {
    /// Take the latest clipboard read the program asked for (clearing it).
    /// Taken every tick whether answered or not, so an ask refused now is
    /// never answered later, when the pane is focused or the setting flips.
    pub fn take_clipboard_ask(&self) -> Option<ClipboardAsk> {
        self.core.take_clipboard_ask()
    }
}

impl HeadlessTerm {
    pub fn take_clipboard_ask(&self) -> Option<ClipboardAsk> {
        self.core.take_clipboard_ask()
    }
}

#[cfg(test)]
#[path = "modelclip_tests.rs"]
mod tests;
