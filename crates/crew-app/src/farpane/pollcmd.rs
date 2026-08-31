//! Draining the `/far` pane's background work each frame: the shell command a
//! panel is running, and the `!` ask waiting on an answer.
//!
//! Split out of [`super`] for the line cap.
use super::FarPane;

impl FarPane {
    /// Whether a command-line command is still running, an AI ask is in
    /// flight, or a remote op is in flight (drives the busy sweep — which is
    /// also what repaints the `thinking… Ns` counter while waiting).
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
            || matches!(self.ask, Some(super::ask::AskState::Thinking { .. }))
            || self.ops_busy()
    }

    /// Drain the running command’s result, if it finished this tick: reload
    /// both panels (the command likely changed the directory contents) and
    /// return a status line for the app to flash.
    pub fn poll_cmd(&mut self) -> Option<String> {
        let (cmd, rx) = self.running.as_ref()?;
        let done = rx.try_recv().ok()?;
        let cmd = cmd.clone();
        self.running = None;
        self.reload_both();
        let outcome = match done.code {
            Some(0) => "ok".to_string(),
            Some(c) => format!("exit {c}"),
            None => "killed".to_string(),
        };
        Some(if done.tail.is_empty() {
            format!("‘{cmd}’ — {outcome}")
        } else {
            format!("‘{cmd}’ — {outcome} · {}", done.tail)
        })
    }

    /// Drain a finished `!` ask, if any: land it (via [`Self::absorb_ask_result`])
    /// or report the worker thread dying without a reply. Returns a status
    /// line for the app to flash, mirroring `poll_cmd`; `None` when nothing
    /// changed this tick (still thinking, or no ask at all).
    pub fn poll_ask(&mut self) -> Option<String> {
        let Some(super::ask::AskState::Thinking { rx, .. }) = &self.ask else {
            return None;
        };
        match rx.try_recv() {
            Ok(res) => Some(self.absorb_ask_result(res)),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.ask = None;
                Some("ask failed: worker died — ! text kept".to_string())
            }
        }
    }

    /// Land a finished ask’s result: a non-blank suggestion replaces
    /// `cmdline` (state becomes `Suggested`, `original` keeps the `!` text
    /// for Esc); a blank suggestion or an error clears `ask` and leaves
    /// `cmdline` untouched. Returns the status line either way.
    pub(crate) fn absorb_ask_result(&mut self, res: Result<String, String>) -> String {
        match res {
            Ok(cmd) if cmd.trim().is_empty() => {
                self.ask = None;
                "no command suggested — ! text kept".to_string()
            }
            Ok(cmd) => {
                let original = std::mem::replace(&mut self.cmdline, cmd.trim().to_string());
                self.ask = Some(super::ask::AskState::Suggested { original });
                "Enter run · Esc discard · keep typing to edit".to_string()
            }
            Err(e) => {
                self.ask = None;
                format!("ask failed: {e} — ! text kept")
            }
        }
    }
}
