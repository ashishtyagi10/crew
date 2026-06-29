//! Builds the left-nav sidebar PaneScene: the StatsPane sections (clock, system,
//! load, host, net, git, LOG) plus the live pane list, framed by a fieldset card
//! whose legend carries the running version — so the build is always visible in
//! the left nav (replacing the old `/about` status flash).
use crew_render::PaneScene;

use crate::app::{CrewApp, GAP};
use crate::chrome;

impl CrewApp {
    /// Push the docked sidebar card onto `scenes`. A no-op when the nav is hidden.
    pub(crate) fn push_sidebar(
        &self,
        scenes: &mut Vec<PaneScene>,
        sh: f32,
        scale: f32,
        cw: f32,
        ch: f32,
    ) {
        if !self.config.show_nav {
            return;
        }
        let sb = chrome::sidebar_rect(sh, self.nav_px(scale), GAP);
        let pane_rows: Vec<crate::panelist::PaneRow> = self
            .panes
            .iter()
            .enumerate()
            .map(|(i, p)| crate::panelist::PaneRow {
                index: i + 1,
                title: p.title_text(),
                focused: i == self.focused,
                activity: p.activity,
            })
            .collect();
        let sidebar = &self.sidebar;
        let log = &self.log;
        let legend = concat!("crew v", env!("CARGO_PKG_VERSION"));
        crate::panecard::push_card(scenes, sb, cw, ch, legend, |cols, rows| {
            sidebar.cells(cols, rows, &pane_rows, log)
        });
    }
}
