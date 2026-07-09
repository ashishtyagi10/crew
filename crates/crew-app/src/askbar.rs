//! The input bar's `?` prefix — ask the AI for a command (à la Warp AI /
//! GitHub Copilot CLI): `?list rust files changed this week` asks the broker's
//! provider for exactly one shell command and drops it into the input bar,
//! ready to edit or Enter. The provider call runs on a worker thread
//! (`crew_plugin::suggest_command` blocks); the winit thread only polls a
//! channel, so the UI never stalls on the network.
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use crate::app::CrewApp;

/// Provider deadline for one ask. Generous for free-tier fallback chains,
/// bounded so a dead network resolves to a status line, not a forever-pending
/// spinner.
const ASK_TIMEOUT: Duration = Duration::from_secs(30);

/// An in-flight ask: the worker thread's result channel.
pub(crate) struct Ask {
    rx: Receiver<Result<String, String>>,
}

/// If `line` is a `?query` ask, return the trimmed query (empty when just
/// `?`); else `None`. Mirrors `bang_command`/`star_command` — an explicit
/// prefix, checked before bare-text routing.
pub(crate) fn qmark_command(line: &str) -> Option<&str> {
    line.strip_prefix('?').map(str::trim)
}

impl CrewApp {
    /// Kick off an ask on a worker thread and flash that it's running. A
    /// second ask while one is pending is refused (one suggestion at a time).
    pub(crate) fn start_ask(&mut self, query: &str) {
        if self.ask.is_some() {
            self.set_status("still asking — one suggestion at a time");
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let q = query.to_string();
        std::thread::spawn(move || {
            let _ = tx.send(crew_plugin::suggest_command(&q, ASK_TIMEOUT));
        });
        self.ask = Some(Ask { rx });
        self.set_status(format!("asking ai for a command — {query}"));
    }

    /// Poll the in-flight ask (called every tick). Returns true when something
    /// changed and the frame should redraw.
    pub(crate) fn poll_ask(&mut self) -> bool {
        let Some(ask) = &self.ask else { return false };
        match ask.rx.try_recv() {
            Ok(res) => {
                self.ask = None;
                self.absorb_ask_result(res);
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.ask = None;
                self.set_status("ask failed: worker died");
                true
            }
        }
    }

    /// Land the suggestion: fill the (still empty) input bar so Enter runs it,
    /// or — if the user typed something new meanwhile — flash it instead of
    /// clobbering their text. Errors go to the status line.
    pub(crate) fn absorb_ask_result(&mut self, res: Result<String, String>) {
        match res {
            Ok(cmd) if cmd.trim().is_empty() => self.set_status("no command suggested"),
            Ok(cmd) => {
                if self.input.text.is_empty() {
                    self.input.text = cmd;
                    self.input.focused = true;
                    self.set_status("suggested — Enter to run, or edit it first");
                } else {
                    // The user typed while the ask ran; never clobber them.
                    self.set_status(format!("suggestion: {cmd}"));
                }
            }
            Err(e) => self.set_status(format!("ask failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qmark_parses_the_query() {
        assert_eq!(qmark_command("?list files"), Some("list files"));
        assert_eq!(qmark_command("?  kill port 8080 "), Some("kill port 8080"));
        assert_eq!(qmark_command("?"), Some(""));
        assert_eq!(qmark_command("ls?"), None);
        assert_eq!(qmark_command("what?"), None);
    }

    #[test]
    fn suggestion_fills_an_empty_bar_ready_to_run() {
        let mut app = CrewApp::default();
        app.absorb_ask_result(Ok("ls -la".into()));
        assert_eq!(app.input.text, "ls -la");
        assert!(app.input.focused, "the bar is focused for the Enter");
        let s = app.active_status().unwrap_or_default();
        assert!(s.contains("Enter"), "status invites the run: {s}");
    }

    #[test]
    fn suggestion_never_clobbers_text_typed_meanwhile() {
        let mut app = CrewApp::default();
        app.input.text = "git st".into();
        app.absorb_ask_result(Ok("ls -la".into()));
        assert_eq!(app.input.text, "git st");
        let s = app.active_status().unwrap_or_default();
        assert!(s.contains("ls -la"), "the suggestion still surfaces: {s}");
    }

    #[test]
    fn empty_suggestion_and_errors_reach_the_status_line() {
        let mut app = CrewApp::default();
        app.absorb_ask_result(Ok("  ".into()));
        assert!(app
            .active_status()
            .unwrap_or_default()
            .contains("no command"));
        app.absorb_ask_result(Err("no AI provider — set DASHSCOPE_API_KEY".into()));
        assert!(app
            .active_status()
            .unwrap_or_default()
            .contains("no AI provider"));
    }
}
