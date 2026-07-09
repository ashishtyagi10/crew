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

/// Scrollback context for one `??` explain: newest lines only, byte-capped so
/// a huge history can't blow the prompt.
const EXPLAIN_LINES: usize = 120;
const EXPLAIN_BYTES: usize = 8 * 1024;

/// What the in-flight ask produces: a command for the bar, or a markdown
/// explanation for the viewer.
enum AskKind {
    Command,
    Explain,
}

/// An in-flight ask: the worker thread's result channel and what to do with
/// the reply when it lands.
pub(crate) struct Ask {
    rx: Receiver<Result<String, String>>,
    kind: AskKind,
}

/// If `line` is a `?query` ask, return the trimmed query (empty when just
/// `?`); else `None`. Mirrors `bang_command`/`star_command` — an explicit
/// prefix, checked before bare-text routing.
pub(crate) fn qmark_command(line: &str) -> Option<&str> {
    line.strip_prefix('?').map(str::trim)
}

/// If `line` is a `??question` explain-this-pane ask, return the trimmed
/// question (empty when just `??` — a default question stands in). Checked
/// BEFORE [`qmark_command`], which would otherwise read `??x` as `?` + `?x`.
pub(crate) fn explain_command(line: &str) -> Option<&str> {
    line.strip_prefix("??").map(str::trim)
}

/// The newest `max_lines` lines of `text`, additionally capped at `max_bytes`
/// (whole lines, oldest dropped first).
pub(crate) fn context_tail(text: &str, max_lines: usize, max_bytes: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut start = lines.len().saturating_sub(max_lines);
    let mut size: usize = lines[start..].iter().map(|l| l.len() + 1).sum();
    while size > max_bytes && start < lines.len() {
        size -= lines[start].len() + 1;
        start += 1;
    }
    lines[start..].join("\n")
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
        self.ask = Some(Ask {
            rx,
            kind: AskKind::Command,
        });
        self.set_status(format!("asking ai for a command — {query}"));
    }

    /// Kick off a `??` explain of the focused terminal's recent output on a
    /// worker thread. Needs a terminal pane to read; refuses a second ask
    /// while one is pending.
    pub(crate) fn start_explain(&mut self, question: &str) {
        if self.ask.is_some() {
            self.set_status("still asking — one suggestion at a time");
            return;
        }
        let Some(pane) = self.panes.get_mut(self.focused) else {
            self.set_status("?? reads a pane — focus a terminal first");
            return;
        };
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let crate::pane::PaneContent::Terminal(t) = &mut pane.content else {
            self.set_status("?? reads a pane — focus a terminal first");
            return;
        };
        let text = crate::dump::capture_scrollback(&mut t.pty, cols, rows);
        let context = context_tail(&text, EXPLAIN_LINES, EXPLAIN_BYTES);
        if context.trim().is_empty() {
            self.set_status("nothing in this pane to explain yet");
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let q = question.to_string();
        std::thread::spawn(move || {
            let _ = tx.send(crew_plugin::explain_output(&context, &q, ASK_TIMEOUT));
        });
        self.ask = Some(Ask {
            rx,
            kind: AskKind::Explain,
        });
        self.set_status("asking ai about this pane…");
    }

    /// Poll the in-flight ask (called every tick). Returns true when something
    /// changed and the frame should redraw.
    pub(crate) fn poll_ask(&mut self) -> bool {
        let Some(ask) = &self.ask else { return false };
        match ask.rx.try_recv() {
            Ok(res) => {
                let explain = matches!(ask.kind, AskKind::Explain);
                self.ask = None;
                if explain {
                    self.absorb_explain_result(res);
                } else {
                    self.absorb_ask_result(res);
                }
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

    /// Land a `??` explanation: write it to a temp markdown file and open the
    /// zoomed viewer on it (the `/md` pane renders headings and code fences).
    pub(crate) fn absorb_explain_result(&mut self, res: Result<String, String>) {
        match res {
            Ok(md) if md.trim().is_empty() => self.set_status("no explanation came back"),
            Ok(md) => {
                let path = std::env::temp_dir().join(format!(
                    "crew-explain-{}.md",
                    crate::chattime::unix_now_ms()
                ));
                match std::fs::write(&path, md) {
                    Ok(()) => self.spawn_md_pane(&path.to_string_lossy()),
                    Err(e) => self.set_status(format!("explain: cannot write {e}")),
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
    fn explain_parses_before_ask() {
        assert_eq!(
            explain_command("??why did this fail"),
            Some("why did this fail")
        );
        assert_eq!(explain_command("?? "), Some(""));
        assert_eq!(explain_command("?one mark"), None);
        assert_eq!(explain_command("ls"), None);
        // a `??` line must never read as a `?` ask for "?why…"
        assert_eq!(qmark_command("??why"), Some("?why"));
    }

    #[test]
    fn explain_result_opens_the_markdown_viewer() {
        let mut app = CrewApp::default();
        app.absorb_explain_result(Ok("## It failed\nBecause of X.".into()));
        let last = app.panes.last().expect("a viewer pane opened");
        assert!(
            matches!(last.content, crate::pane::PaneContent::Markdown(_)),
            "the answer opens in the md viewer"
        );
        assert!(app.zoomed, "the viewer opens zoomed");
    }

    #[test]
    fn explain_errors_reach_the_status_line() {
        let mut app = CrewApp::default();
        app.absorb_explain_result(Err("no AI provider".into()));
        assert!(app
            .active_status()
            .unwrap_or_default()
            .contains("no AI provider"));
        assert!(app.panes.is_empty(), "no pane on error");
    }

    #[test]
    fn context_tail_bounds_lines_and_bytes() {
        let many: String = (0..500).map(|i| format!("line {i}\n")).collect();
        let tail = context_tail(&many, 120, 8 * 1024);
        assert!(tail.lines().count() <= 120);
        assert!(
            tail.ends_with("line 499"),
            "keeps the newest lines: …{tail:.20}"
        );
        let fat = "x".repeat(100_000);
        assert!(context_tail(&fat, 120, 8 * 1024).len() <= 8 * 1024);
    }

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
