//! Driving the OS window title from the focused pane.
use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// The OS window title for the focused pane (terminal title, or a label),
    /// falling back to "Crew".
    pub(crate) fn focused_title(&self) -> String {
        match self.panes.get(self.focused) {
            Some(p) => match &p.content {
                PaneContent::Terminal(t) => {
                    let ti = t.pty.title();
                    if ti.is_empty() {
                        "Crew".into()
                    } else {
                        format!("{ti} — Crew")
                    }
                }
                PaneContent::Chat(_) => "Chat — Crew".into(),
                PaneContent::Settings(_) => "Settings — Crew".into(),
            },
            None => "Crew".into(),
        }
    }

    /// Update the OS window title when the focused pane's title changes.
    pub(crate) fn sync_window_title(&mut self) {
        let title = self.focused_title();
        if title != self.win_title {
            self.win_title = title.clone();
            if let Some(w) = &self.window {
                w.set_title(&title);
            }
        }
    }
}
