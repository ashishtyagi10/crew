//! Opening one of crew's own panes — settings, todo, usage, disk, dash, far,
//! goal, batch, swarm.
//!
//! Only the TODO list refuses to open twice, and that is a consequence rather
//! than a rule: Esc minimizes it instead of closing it, so a `/todo` that
//! pushed a fresh one would strand the pane you put away.
//!
//! The rest are VISITS — open, do a thing, leave — which is why settings and
//! far open ZOOMED: far has two panes of its own to fit and a settings form
//! is a column to read down, and neither wants a quarter tile beside the work
//! it interrupts. Nothing new gets you out: Esc already means "leave" here,
//! and `close_pane` clears the zoom, so you land on the grid you left.
//!
//! Split from [`crate::spawn`] for the line cap, along the line between
//! spawning a TERMINAL (a process, a PTY, a shell) and opening a pane crew
//! draws itself.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::farpane::FarPane;
use crate::pane::{Pane, PaneContent};
use crate::settingspane::SettingsPane;
use crate::spawn::PLACEHOLDER_RECT;
use std::path::Path;

impl CrewApp {
    /// Spawn a settings pane showing the app config and focus it.
    pub(crate) fn spawn_settings_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let families = self
            .renderer
            .as_mut()
            .map(|r| r.monospace_families())
            .unwrap_or_default();
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Settings(SettingsPane::new(self.config.clone(), families)),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        self.focus_new_pane();
        self.zoomed = true;
    }

    /// Open the todo list and focus it — or bring back the one already open,
    /// since Esc MINIMIZES this pane (`TodoAction::Minimize`) and opening is
    /// how you get it back. Returns its index: `spawn_todo_pane_done` can no
    /// longer assume the pane it wants is the last one.
    pub(crate) fn spawn_todo_pane(&mut self) -> usize {
        if let Some(i) = self
            .panes
            .iter()
            .position(|p| matches!(p.content, PaneContent::Todo(_)))
        {
            // Focusing is what restores a minimized pane (`reconcile_grid`).
            self.focused = i;
            self.input.focused = false;
            return i;
        }
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Todo(crate::todopane::TodoPane::new()),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        self.focus_new_pane();
        self.panes.len() - 1
    }

    /// Spawn a todo pane already open on the done-history view (`/todo
    /// done`), optionally pre-filtered to one `@project` or one `#assignee`.
    pub(crate) fn spawn_todo_pane_done(&mut self, filter: Option<(char, String)>) {
        let i = self.spawn_todo_pane();
        if let Some(PaneContent::Todo(t)) = self.panes.get_mut(i).map(|p| &mut p.content) {
            match filter {
                Some((crate::todopane::parse::WHO, name)) => t.who = Some(name),
                Some((_, name)) => t.filter = Some(name),
                None => {}
            }
            t.set_done_view(true);
        }
    }

    /// Spawn a Far dual-pane file-manager pane rooted at Crew's cwd, and focus it.
    pub(crate) fn spawn_far_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let cwd = self
            .spawn_cwd()
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Far(FarPane::new(cwd)),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        self.focus_new_pane();
        self.zoomed = true;
    }
}

#[cfg(test)]
#[path = "spawnpanes_tests.rs"]
mod tests;
