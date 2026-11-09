//! The `scrollback` setting: how many lines a terminal pane keeps above its
//! screen. alacritty's 10 000 was fixed in code, so a long build log or a
//! chatty agent lost its start with no way to keep more, and a small machine
//! had no way to keep less.
use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Hand every terminal pane the configured length — on spawn (a new
    /// pane starts at alacritty's default) and on a Settings save, so open
    /// panes shrink or grow at once rather than only the next one opened.
    /// A shrink throws the oldest lines away; `set_scrollback` counts them
    /// so the pictures above stay on their text.
    pub(crate) fn apply_scrollback(&mut self) {
        let lines = self.config.scrollback;
        for p in &mut self.panes {
            if let PaneContent::Terminal(t) = &mut p.content {
                t.pty.set_scrollback(lines);
            }
        }
    }
}

#[cfg(test)]
#[path = "scrollback_tests.rs"]
mod tests;
