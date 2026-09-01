//! Opening one of crew's own panes — settings, todo, usage, disk, dash, far,
//! goal, batch, swarm — each of which focuses an existing one rather than
//! opening a second.
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
    }

    /// Spawn the todo-list pane (the shared store loads on open) and focus it.
    pub(crate) fn spawn_todo_pane(&mut self) {
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
    }

    /// Spawn a todo pane already open on the done-history view (`/todo
    /// done`), optionally pre-filtered to one `@project`.
    pub(crate) fn spawn_todo_pane_done(&mut self, filter: Option<String>) {
        self.spawn_todo_pane();
        if let Some(PaneContent::Todo(t)) = self.panes.last_mut().map(|p| &mut p.content) {
            t.filter = filter;
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
    }

    /// Plan `goal` into a task graph off-thread and run it in a swarm pane. An
    /// empty goal just shows a usage hint (no pane).
    pub(crate) fn spawn_goal_pane(&mut self, goal: &str) {
        let goal = goal.trim();
        if goal.is_empty() {
            self.set_status("usage: /goal <text>");
            return;
        }
        self.push_swarm_pane(crate::swarmpane::SwarmPane::for_goal(goal.to_string()));
    }

    /// Run a batch of jobs read from a file (one job per line) as an all-parallel
    /// swarm. An empty path shows a usage hint; an unreadable/empty file reports
    /// why instead of opening an empty pane.
    pub(crate) fn spawn_batch_pane(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            self.set_status("usage: /batch <file> (one job per line)");
            return;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                self.set_status(format!("batch: cannot read {path}: {e}"));
                return;
            }
        };
        let jobs = crate::swarmpane::jobs_from_lines(&text);
        if jobs.is_empty() {
            self.set_status(format!("batch: no jobs in {path}"));
            return;
        }
        let n = jobs.len();
        match crate::swarmpane::SwarmPane::for_batch(jobs) {
            Ok(swarm) => {
                self.push_swarm_pane(swarm);
                self.set_status(format!("batch: running {n} jobs"));
            }
            Err(e) => self.set_status(format!("batch: {e}")),
        }
    }

    /// Push a swarm pane into the grid and focus it.
    fn push_swarm_pane(&mut self, swarm: crate::swarmpane::SwarmPane) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Swarm(swarm),
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
        self.redraw();
    }
}
