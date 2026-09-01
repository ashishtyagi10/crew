//! Opening the three panes that READ something and draw it — usage, disk and
//! dash. Each starts a scan or a ledger read on opening, which is what sets
//! them apart from the panes that simply appear.
//!
//! Split from [`crate::spawnpanes`] for the line cap.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;

impl CrewApp {
    /// Spawn the `/usage` pane — seven days of spend, drawn.
    pub(crate) fn spawn_usage_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Usage(crate::usagepane::UsagePane::new()),
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

    /// Spawn the `/disk` pane — a treemap of `dir` (the pane's cwd by
    /// default), scanned on a worker thread.
    pub(crate) fn spawn_disk_pane(&mut self, dir: Option<std::path::PathBuf>) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let root = dir.unwrap_or_else(|| self.cwd.clone());
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Disk(crate::diskpane::DiskPane::new(root)),
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

    /// Spawn the `/dash` pane — the machine and the week, drawn.
    pub(crate) fn spawn_dash_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Dash(crate::dashpane::DashPane::new()),
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
}
