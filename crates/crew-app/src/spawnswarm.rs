//! Starting WORK in a pane: `/goal` plans a task graph and runs it, `/batch`
//! runs a file of jobs, and both land in the swarm pane this pushes.
//!
//! Split from [`crate::spawnpanes`] at the line the cap found, and it is a
//! real one: that file opens a surface for you to LOOK at — settings, the
//! todo list, a file manager — and this one sets something running and shows
//! you it happening.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;

impl CrewApp {
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
