//! Builds the left-nav sidebar PaneScene: the StatsPane sections (clock, system,
//! load, host, net, git, LOG) plus the live pane list, framed by a fieldset card
//! whose legend carries the running version — so the build is always visible in
//! the left nav (replacing the old `/about` status flash).
use crew_render::PaneScene;

use crate::app::{CrewApp, GAP};
use crate::chrome;
use crate::layout::Rect;

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
        let full = chrome::sidebar_rect(sh, self.nav_px(scale), GAP);
        // While a `/update` runs, dock a distinct UPDATE card on top of the stats
        // card, shrinking the stats card below it (chrome::stats_card_rect — the
        // same rect the PANES hit-test uses). It vanishes once the update ends.
        if let Some(u) = &self.update {
            let top = Rect {
                h: (chrome::UPDATE_CARD_ROWS * ch).min(full.h),
                ..full
            };
            crate::panecard::push_card(scenes, top, cw, ch, "UPDATE", |cols, rows| {
                crate::updatecard::update_cells(u, cols, rows)
            });
        }
        let sb = chrome::stats_card_rect(sh, self.nav_px(scale), GAP, ch, self.update.is_some());
        let pane_rows: Vec<crate::panelist::PaneRow> = self
            .panes
            .iter()
            .enumerate()
            .map(|(i, p)| crate::panelist::PaneRow {
                index: i + 1,
                title: p.title_text(),
                focused: i == self.focused,
                activity: p.activity,
                minimized: p.hidden,
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
