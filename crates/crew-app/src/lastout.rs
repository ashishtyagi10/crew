//! `/out`: the last command's output, on its own, in the file viewer.
//!
//! A long build's output is buried in a scrollback the moment the prompt comes
//! back — mixed in with what you ran before it and whatever the shell printed
//! after. Crew already knows where that command's output starts and ends
//! ([`crate::cmdspan`]), so this slices exactly those lines out and opens them
//! in the viewer, where they can be scrolled, searched, walked by `]`/`[` and
//! kept while you carry on working in the pane.
use std::path::PathBuf;

use crate::app::CrewApp;
use crate::cmdspan::Spans;
use crate::pane::PaneContent;

/// The file one pane's captured output is written to. Per pane index, so
/// running `/out` twice in the same pane overwrites rather than litters.
pub(crate) fn temp_path(pane: usize, name: &str) -> PathBuf {
    let slug: String = name
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(24)
        .collect();
    let slug = if slug.is_empty() { "out".into() } else { slug };
    std::env::temp_dir().join(format!("crew-out-{pane}-{slug}.log"))
}

/// The `[from, to)` slice of `lines`, as text. Clamped, since the scrollback
/// the span was measured against may have scrolled away since.
pub(crate) fn slice(lines: &[String], from: usize, to: usize) -> String {
    let to = to.min(lines.len());
    let from = from.min(to);
    lines[from..to].join("\n")
}

impl CrewApp {
    /// The focused pane's last command output as `(command, text)`, or why
    /// there is none. Shared by `/out`, which opens it, and `/copy out`,
    /// which hands it to the clipboard — the same slice either way.
    pub(crate) fn last_output(&mut self) -> Result<(String, String), String> {
        let focused = self.focused;
        let Some(pane) = self.panes.get_mut(focused) else {
            return Err("no pane focused".into());
        };
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let PaneContent::Terminal(t) = &mut pane.content else {
            return Err("not a terminal pane".into());
        };
        let now = t.pty.scrollable_lines();
        let Some((name, from, to)) = t.spans.latest(now).map(|s| {
            let (from, to) = Spans::range(s, now);
            (s.name.clone(), from, to)
        }) else {
            return Err("nothing has run in this pane yet".into());
        };
        // The whole buffer, once, then the slice: paging the terminal is the
        // expensive part and doing it per line would be quadratic.
        let text = crate::dump::capture_scrollback(&mut t.pty, cols, rows);
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        // `capture_scrollback` caps what it reads, so the line numbering it
        // returns can start later than the buffer's own. Anchor on the end.
        let drop = now.saturating_sub(lines.len());
        let body = slice(&lines, from.saturating_sub(drop), to.saturating_sub(drop));
        match body.trim().is_empty() {
            true => Err(format!("{name} printed nothing")),
            false => Ok((name, body)),
        }
    }

    /// `/out` — open the focused pane's last command output in the viewer.
    pub(crate) fn open_last_output(&mut self) {
        let focused = self.focused;
        let (name, body) = match self.last_output() {
            Ok(v) => v,
            Err(why) => {
                self.set_status(format!("out: {why}"));
                return;
            }
        };
        let lines = body.lines().count();
        let path = temp_path(focused, &name);
        if let Err(e) = std::fs::write(&path, format!("{body}\n")) {
            self.set_status(format!("out: cannot write: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.name_last_view(&format!("out \u{b7} {name}"));
        self.mark_last_view_ephemeral(before);
        self.set_status(format!("{name}: {lines} lines"));
    }
}

#[cfg(test)]
#[path = "lastout_tests.rs"]
mod tests;
