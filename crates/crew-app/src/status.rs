//! Transient status messages flashed on the input card's bottom border (e.g.
//! "copied 12 lines", "cd: no such directory"), auto-expiring after a few
//! seconds so the bar normally stays clean.
use std::time::{Duration, Instant};

use crate::app::CrewApp;
use crate::applog::{LogEntry, LogLevel};

/// How long a status message stays visible.
const STATUS_TTL: Duration = Duration::from_secs(3);

/// Most entries kept in the live LOG ring buffer (oldest dropped past this).
pub(crate) const LOG_CAP: usize = 64;

impl CrewApp {
    /// Flash a transient status message and request a redraw. The message is also
    /// appended (with an `HH:MM` timestamp) to the live LOG ring buffer shown in
    /// the left nav — unlike the 3-second flash, the log keeps a scrollback of
    /// recent activity. The flash itself stays untimestamped, so the input bar
    /// reads cleanly.
    pub(crate) fn set_status(&mut self, msg: impl Into<String>) {
        self.set_status_level(LogLevel::Info, msg);
    }

    /// [`Self::set_status`] at error level: same flash, but the LOG entry
    /// renders in the attention color so failures stand out in the tail.
    pub(crate) fn set_status_err(&mut self, msg: impl Into<String>) {
        self.set_status_level(LogLevel::Error, msg);
    }

    /// The one writer behind both levels (and the poll-tick drain of
    /// [`crate::applog::AppLog`]).
    pub(crate) fn set_status_level(&mut self, level: LogLevel, msg: impl Into<String>) {
        let msg = msg.into();
        if self.log.len() >= LOG_CAP {
            self.log.remove(0);
        }
        let text = format!("{} {}", log_stamp(), msg);
        // Mirror every entry into the on-disk session log (`/log`).
        crate::activitylog::append(level, &text);
        self.log.push(LogEntry { level, text });
        // Errors also step onto the canvas as an alert toast; routine statuses
        // ("copied 12 lines") stay a quiet flash on the bar.
        if level == LogLevel::Error {
            self.toasts
                .push(msg.clone(), "error", true, crate::anim::now_ms());
        }
        self.status = Some((msg, Instant::now()));
        self.redraw();
    }

    /// Surface a pane event through the notification system: gated by the config
    /// toggles, throttled by the [`crate::notify::Notifier`], then flashed on the
    /// input bar and appended to the LOG (via [`Self::set_status`]). In-app only —
    /// no OS notifications.
    pub(crate) fn notify(&mut self, kind: crate::notify::NotifyKind, pane: String, detail: String) {
        use crate::notify::NotifyKind;
        if !self.config.notify {
            return;
        }
        let enabled = match kind {
            NotifyKind::AgentDone => self.config.notify_agent_done,
            NotifyKind::Bell => self.config.notify_bell,
            NotifyKind::Exited => self.config.notify_exit,
            // Patterns are opt-in: they only fire when the user lists them.
            NotifyKind::Pattern => true,
            // Waiting rides the master `notify` switch alone: the detection
            // is already conservative, and the badge is the primary surface.
            NotifyKind::Waiting => true,
        };
        if !enabled {
            return;
        }
        if let Some(msg) = self
            .notifier
            .record(kind, pane, detail, std::time::Instant::now())
        {
            // The same line, twice as loud: a toast card on the canvas plus
            // the usual flash + LOG entry. Waiting/exited border in the bell
            // color — those are the two that want the user back.
            let (legend, alert) = match kind {
                NotifyKind::AgentDone => ("done", false),
                NotifyKind::Bell => ("bell", false),
                NotifyKind::Pattern => ("match", false),
                NotifyKind::Exited => ("exited", true),
                NotifyKind::Waiting => ("waiting", true),
            };
            self.toasts
                .push(msg.clone(), legend, alert, crate::anim::now_ms());
            self.set_status(msg);
        }
    }

    /// Push the configured watch patterns onto every terminal pane's PTY scanner.
    /// Called after spawning a pane and whenever the patterns change (`/notify`,
    /// settings save), so all live panes share the current watch list.
    pub(crate) fn apply_notify_patterns(&mut self) {
        let patterns = self.config.notify_patterns.clone();
        for p in &mut self.panes {
            if let crate::pane::PaneContent::Terminal(t) = &mut p.content {
                t.pty.set_watch_patterns(&patterns);
            }
        }
    }

    /// Clear the live LOG ring buffer (`/clearlog`), then note the action so the
    /// sidebar shows the log was reset rather than going blank.
    pub(crate) fn clear_log(&mut self) {
        self.log.clear();
        self.set_status("activity log cleared");
    }

    /// The current status text, or `None` once it has expired.
    pub(crate) fn active_status(&self) -> Option<&str> {
        self.status
            .as_ref()
            .filter(|(_, t)| t.elapsed() < STATUS_TTL)
            .map(|(s, _)| s.as_str())
    }

    /// Drop an expired status; returns `true` when one was cleared (so the
    /// caller knows to repaint the now-empty bottom border).
    pub(crate) fn expire_status(&mut self) -> bool {
        let expired = self
            .status
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed() >= STATUS_TTL);
        if expired {
            self.status = None;
        }
        expired
    }
}

/// `HH:MM` stamp prefixed onto each LOG entry, from the wall clock.
fn log_stamp() -> String {
    let (time, _) = crate::clock::now_strings();
    time.get(..5).unwrap_or(&time).to_string()
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn log_entry_is_timestamped_but_flash_is_not() {
        let mut app = CrewApp::default();
        app.set_status("hello world");
        // The input-bar flash is the bare message…
        assert_eq!(app.active_status(), Some("hello world"));
        // …while the LOG entry carries an `HH:MM` stamp before it.
        let last = app.log.last().expect("log has the entry");
        assert!(last.text.ends_with("hello world"));
        assert!(last.text.contains(':') && last.text != "hello world");
        assert_eq!(last.level, crate::applog::LogLevel::Info);
    }

    #[test]
    fn error_status_flags_its_log_entry() {
        let mut app = CrewApp::default();
        app.set_status_err("broker fell over");
        // Same flash as an info status…
        assert_eq!(app.active_status(), Some("broker fell over"));
        // …but the LOG entry carries the error level for the renderer.
        assert_eq!(
            app.log.last().unwrap().level,
            crate::applog::LogLevel::Error
        );
    }

    #[test]
    fn clear_log_empties_then_notes_the_reset() {
        let mut app = CrewApp::default();
        app.set_status("a");
        app.set_status("b");
        assert_eq!(app.log.len(), 2);
        app.clear_log();
        // Cleared down to just the single "cleared" note (not blank).
        assert_eq!(app.log.len(), 1);
        assert!(app.log[0].text.ends_with("activity log cleared"));
    }

    #[test]
    fn notify_logs_a_flash_when_enabled() {
        use crate::notify::NotifyKind;
        let mut app = CrewApp::default();
        app.notify(NotifyKind::AgentDone, "crew".into(), "claude".into());
        assert_eq!(app.active_status(), Some("✓ claude finished in crew"));
        assert!(app
            .log
            .last()
            .unwrap()
            .text
            .contains("claude finished in crew"));
    }

    #[test]
    fn notify_respects_the_per_kind_toggle() {
        use crate::notify::NotifyKind;
        let mut app = CrewApp::default();
        app.config.notify_bell = false;
        let before = app.log.len();
        app.notify(NotifyKind::Bell, "crew".into(), String::new());
        assert_eq!(app.log.len(), before, "bell notifications are disabled");
    }

    #[test]
    fn notify_master_switch_suppresses_everything() {
        use crate::notify::NotifyKind;
        let mut app = CrewApp::default();
        app.config.notify = false;
        let before = app.log.len();
        app.notify(NotifyKind::Exited, "crew".into(), String::new());
        assert_eq!(
            app.log.len(),
            before,
            "master switch off → no notifications"
        );
    }
}
