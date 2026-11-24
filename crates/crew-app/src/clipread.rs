//! OSC 52 clipboard READS: a program in a pane asking what was copied
//! (`ESC ] 52 ; c ; ? BEL`), the way nvim's paste provider over ssh and
//! tmux's `set-clipboard` get at the clipboard of the machine you sit at.
//!
//! Reading is a privacy risk, not a convenience: any program in any pane, or
//! bytes `cat`ed from a file, could ask. So it is the `clipboard_read`
//! setting, `deny` by default (Ghostty asks, kitty leaves it out), and even
//! when allowed only the pane holding the keys is answered (the focused pane,
//! not the input bar, in a window KNOWN to be focused) — a background pane
//! reading your clipboard is the abuse case. Unlike everywhere else, a window
//! whose focus is still unknown counts as unfocused: a background launch or a
//! restored shell's startup script must not read before the first `Focused`.
//! The gate is checked here at reply time, so a changed setting holds for the
//! panes already open. `ask` has no prompt yet and reads as `deny`.
//!
//! One pane is answered at most once per [`READ_GAP`], and never while its
//! last answer still sits unread in its input queue: a program that asks in a
//! loop without reading would otherwise stack 1.4 MB copies in crew's memory.
//!
//! Known limitation: the answer is sent a tick after the ask, so a DSR or DA
//! query the program sent AFTER its read can be answered BEFORE it.
use std::collections::HashMap;
use std::io::Write;
use std::time::{Duration, Instant};

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

/// The least time between two answers to one pane — about four reads a
/// second, plenty for a paste and too slow to watch the clipboard.
pub(crate) const READ_GAP: Duration = Duration::from_millis(250);

/// When each pane (by its shell's PID, which outlives index shifts on close)
/// was last let through the gate.
pub(crate) type LastReads = HashMap<u32, Instant>;

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
        self.serve_clipboard_asks(&mut crate::clipboard::system_text, Instant::now());
    }

    /// Take every terminal pane's pending read and answer the ones the gate
    /// lets through from `read`, at time `now`. The rest are dropped, never
    /// kept: an ask refused now must not be answered once its pane gains
    /// focus, nor one asked too soon after the last.
    pub(crate) fn serve_clipboard_asks(
        &mut self,
        read: &mut dyn FnMut() -> Option<String>,
        now: Instant,
    ) {
        let allowed = self.config.clipboard_read_allowed();
        let (focus, win, bar) = (self.focused, self.win_focus, self.input.focused);
        let last = &mut self.clip_reads;
        last.retain(|_, at| now.saturating_duration_since(*at) < READ_GAP);
        for (i, pane) in self.panes.iter_mut().enumerate() {
            let PaneContent::Terminal(t) = &mut pane.content else {
                continue;
            };
            let Some(ask) = t.pty.take_clipboard_ask() else {
                continue;
            };
            let keys = win == Some(true) && crate::focuspush::has_keys(i, focus, win, bar);
            let Some(pid) = t.pty.shell_pid() else {
                continue;
            };
            let due = !last.contains_key(&pid) && t.pty.input_backlog() == 0;
            if allowed && keys && due {
                // Stamped on passing, not on replying: an over-cap clipboard
                // is not re-read on every ask either.
                last.insert(pid, now);
            }
            if let Some(reply) = answer(&ask, allowed, keys && due, read) {
                let _ = t.input.write_all(reply.as_bytes());
                let _ = t.input.flush();
            }
        }
    }
}

#[cfg(test)]
#[path = "clipreadgate_tests.rs"]
mod gate_tests;
#[cfg(test)]
#[path = "clipreadpane_tests.rs"]
mod pane_tests;
#[cfg(test)]
#[path = "clipread_tests.rs"]
mod tests;
