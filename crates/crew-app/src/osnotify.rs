//! The OS banner for a job that ended while crew was out of sight. In-app
//! notices (`notify.rs`) only help if the window is in front; start
//! `cargo test`, switch to the browser, and the toast is drawn for nobody. So
//! when the window has lost focus, a finished command or agent turn also posts
//! one system banner. The throttle and the `notify` / `notify_agent_done`
//! switches already ran by the time this is asked, and the min-seconds floor
//! (`notify_min_secs`) is applied where the finish is detected (`poll.rs`,
//! `turndone.rs`), so a short `ls` never gets here.
//!
//! Delivery is a child process, never a library: macOS `osascript` (the
//! text travels as argv, so nothing is interpolated into the AppleScript),
//! Linux `notify-send` when it is installed, Windows nothing. A banner cannot
//! focus the pane when clicked — `osascript` has no click callback.
use crate::notify::NotifyKind;

/// Longest command text shown: a banner is read at a glance, and a command
/// line can carry a token or a long path nobody needs on the lock screen.
const CMD_MAX: usize = 60;

/// One banner: the pane's name over a short line about how the job ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Banner {
    pub title: String,
    pub body: String,
}

/// Whether this event deserves a banner, and its words. Only a finished
/// command / agent turn, only when the OS says the window is NOT focused
/// (`None` — not told yet — counts as focused, as everywhere else), only when
/// the `notify_os` setting is on.
pub(crate) fn decide(
    kind: NotifyKind,
    win_focus: Option<bool>,
    enabled: bool,
    pane: &str,
    detail: &str,
) -> Option<Banner> {
    let ended = matches!(kind, NotifyKind::AgentDone | NotifyKind::Failed);
    (enabled && ended && win_focus == Some(false)).then(|| Banner {
        title: clean(pane, 80),
        body: body(kind, detail),
    })
}

/// `cargo test — failed (exit 101) · 42s`, `cargo test — done · 42s`, or
/// `agent turn — done · 1m12`. The detail is the in-app wording
/// (`cmd (42s) — exit 101`, `turn (1m12)`); this re-cuts it for a banner.
fn body(kind: NotifyKind, detail: &str) -> String {
    let (head, code) = match detail.rsplit_once(" \u{2014} exit ") {
        Some((h, c)) => (h, Some(c.trim())),
        None => (detail, None),
    };
    let (cmd, took) = match head.strip_suffix(')').and_then(|h| h.rsplit_once(" (")) {
        Some((c, t)) => (c, Some(t)),
        None => (head, None),
    };
    let cmd = if cmd == "turn" { "agent turn" } else { cmd };
    let how = match (kind, code) {
        (NotifyKind::Failed, Some(c)) => format!("failed (exit {})", clean(c, 8)),
        (NotifyKind::Failed, None) => "failed".to_string(),
        _ => "done".to_string(),
    };
    let tail = took.map(|t| format!(" \u{b7} {}", clean(t, 12)));
    format!(
        "{} \u{2014} {how}{}",
        clean(cmd, CMD_MAX),
        tail.unwrap_or_default()
    )
}

/// One line, no control characters, at most `max` chars (then `…`).
fn clean(s: &str, max: usize) -> String {
    let one: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let one = one.trim();
    match one.char_indices().nth(max) {
        Some((i, _)) => format!("{}\u{2026}", one[..i].trim_end()),
        None => one.to_string(),
    }
}

/// Escape for inside an AppleScript double-quoted string. Delivery passes the
/// text as argv and does not need this; kept for [`script_inline`], the form
/// that is easy to read in a test and proves the escaping holds regardless.
#[cfg(test)]
pub(crate) fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The AppleScript lines (one per `-e`) that show argv's body and title.
const SCRIPT: [&str; 3] = [
    "on run argv",
    "display notification (item 1 of argv) with title (item 2 of argv)",
    "end run",
];

impl crate::app::CrewApp {
    /// Post the banner for an event that already passed the switches and the
    /// throttle, if the window is out of sight (see [`decide`]).
    pub(crate) fn os_banner(&self, kind: NotifyKind, pane: &str, detail: &str) {
        let focus = self.win_focus;
        if let Some(b) = decide(kind, focus, self.config.notify_os, pane, detail) {
            post(b);
        }
    }
}

/// Post the banner without blocking the UI thread. Tests record it instead.
pub(crate) fn post(b: Banner) {
    #[cfg(test)]
    sent::push(b);
    #[cfg(not(test))]
    {
        let _ = std::thread::Builder::new()
            .name("os-notify".into())
            .spawn(move || deliver(&b));
    }
}

#[cfg(not(test))]
fn deliver(b: &Banner) {
    use crew_hive::childproc::no_console_window;
    use std::process::{Command, Stdio};
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("osascript");
        no_console_window(&mut c);
        for line in SCRIPT {
            c.args(["-e", line]);
        }
        // `--` so a title that starts with `-` is never read as an option.
        c.args(["--", &b.body, &b.title]);
        c
    } else if cfg!(target_os = "linux") {
        let mut c = Command::new("notify-send");
        no_console_window(&mut c);
        c.args(["-a", "crew", "--", &b.title, &b.body]);
        c
    } else {
        return; // Windows: no banner (no dependency-free way to post one).
    };
    // A missing `notify-send` is the normal Linux case: nothing to report.
    let _ = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// The seam: under test `post` lands here instead of reaching the OS.
#[cfg(test)]
pub(crate) mod sent {
    use super::Banner;
    use std::cell::RefCell;
    thread_local!(static LOG: RefCell<Vec<Banner>> = const { RefCell::new(Vec::new()) });
    pub(super) fn push(b: Banner) {
        LOG.with(|l| l.borrow_mut().push(b));
    }
    pub(crate) fn take() -> Vec<Banner> {
        LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
    }
}

#[cfg(test)]
#[path = "osnotify_tests.rs"]
mod tests;
