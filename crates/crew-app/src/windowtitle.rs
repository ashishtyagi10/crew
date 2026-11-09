//! Driving the OS window title from the focused pane.
use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// The OS window title for the focused pane (terminal title, or a label),
    /// falling back to "Crew". A pane that has a name — agent smith, or one
    /// you gave with `/name` — is called that: the smith pane read `Chat —
    /// Crew`, a word its own legend never uses.
    ///
    /// With more than one tab the active tab is named too, with where it
    /// sits (`zsh — web (2/3) — Crew`): with the nav hidden nothing else on
    /// screen says which tab you are in. One tab keeps the plain title.
    pub(crate) fn focused_title(&self) -> String {
        let pane = self.focused_label();
        let n = self.tabs.list.len();
        let tab = (n > 1)
            .then(|| {
                let at = self.tabs.pos(self.tabs.active)? + 1;
                let name = self.tab_title(self.tabs.active);
                // An unnamed tab is called after its first pane; when that
                // is the focused one, saying it twice tells nothing.
                Some(if pane.as_deref() == Some(name.as_str()) {
                    format!("({at}/{n})")
                } else {
                    format!("{name} ({at}/{n})")
                })
            })
            .flatten();
        let mut parts: Vec<String> = pane.into_iter().chain(tab).collect();
        parts.push("Crew".into());
        parts.join(" — ")
    }

    /// What the focused pane is called in the title, `None` when it has
    /// nothing to say (no pane, or a shell that never set a title).
    fn focused_label(&self) -> Option<String> {
        let p = self.panes.get(self.focused)?;
        if p.name.is_some() && !matches!(p.content, PaneContent::Terminal(_)) {
            return Some(p.title_text());
        }
        let s = match &p.content {
            PaneContent::Terminal(t) => {
                let ti = t.pty.title();
                return (!ti.is_empty()).then(|| ti.to_string());
            }
            PaneContent::View(_) => return Some(p.title_text()),
            PaneContent::Chat(_) => "Chat",
            PaneContent::Settings(_) => "Settings",
            PaneContent::Far(_) => "Far",
            PaneContent::Swarm(_) => "Swarm",
            PaneContent::Todo(_) => "Todo",
            PaneContent::Usage(_) => "Usage",
            PaneContent::Disk(_) => "Disk",
            PaneContent::Dash(_) => "Dashboard",
            PaneContent::Inbox(_) => "Inbox",
        };
        Some(s.into())
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

#[cfg(test)]
#[path = "windowtitle_tests.rs"]
mod tests;
