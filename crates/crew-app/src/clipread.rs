//! OSC 52 clipboard READS: a program in a pane asking what was copied
//! (`ESC ] 52 ; c ; ? BEL`), the way nvim's paste provider over ssh and
//! tmux's `set-clipboard` get at the clipboard of the machine you sit at.
//!
//! Reading is a privacy risk, not a convenience: any program in any pane, or
//! bytes `cat`ed from a file, could ask. So it is the `clipboard_read`
//! setting, `deny` by default (Ghostty asks, kitty leaves it out), and even
//! when allowed only the FOCUSED pane of a focused window is answered — a
//! background pane reading your clipboard is the abuse case. The gate is
//! checked here at reply time, so a changed setting holds for the panes
//! already open. `ask` has no prompt yet and reads as `deny`.
use std::io::Write;

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::ClipboardAsk;

/// The setting's values, in the form's cycle order.
pub(crate) const VALUES: [&str; 2] = ["deny", "allow"];

/// The most clipboard a program is handed, in bytes of text. Past it the
/// answer is nothing at all rather than a cut: a truncated clipboard pasted
/// as if whole is worse than no paste, and a huge one (≈1.4× in base64)
/// would sit in the pane's input queue.
pub(crate) const READ_CAP: usize = 1 << 20;

/// The reply owed to `ask`, or `None` for silence. The clipboard is read
/// only once the setting and the focus have both said yes; an empty or
/// non-text clipboard answers empty, so the program is not left waiting.
pub(crate) fn answer(
    ask: &ClipboardAsk,
    allowed: bool,
    focused: bool,
    read: &mut dyn FnMut() -> Option<String>,
) -> Option<String> {
    if !allowed || !focused {
        return None;
    }
    let text = read().unwrap_or_default();
    (text.len() <= READ_CAP).then(|| ask.reply(&text))
}

impl CrewApp {
    /// Each tick: honour OSC 52 copies, and answer (or refuse) reads.
    pub(crate) fn serve_pane_clipboard(&mut self) {
        if let Some(text) = self.take_pane_clipboard() {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(text);
            }
        }
        self.serve_clipboard_asks(&mut crate::clipboard::system_text);
    }

    /// Take every terminal pane's pending read and answer the ones the gate
    /// lets through from `read`. The rest are dropped, never kept: an ask
    /// refused now must not be answered once its pane gains focus.
    pub(crate) fn serve_clipboard_asks(&mut self, read: &mut dyn FnMut() -> Option<String>) {
        let allowed = self.config.clipboard_read_allowed();
        let window = self.win_focus != Some(false);
        let focus = self.focused;
        for (i, pane) in self.panes.iter_mut().enumerate() {
            let PaneContent::Terminal(t) = &mut pane.content else {
                continue;
            };
            let Some(ask) = t.pty.take_clipboard_ask() else {
                continue;
            };
            if let Some(reply) = answer(&ask, allowed, window && i == focus, read) {
                let _ = t.input.write_all(reply.as_bytes());
                let _ = t.input.flush();
            }
        }
    }
}

#[cfg(test)]
#[path = "clipreadpane_tests.rs"]
mod pane_tests;
#[cfg(test)]
#[path = "clipread_tests.rs"]
mod tests;
