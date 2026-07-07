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
        let mut sb = chrome::sidebar_rect(sh, self.nav_px(scale), GAP);
        // While a `/update` runs, dock a distinct UPDATE card on top of the stats
        // card (a 4-row fieldset: 2 border + 2 content rows), shrinking the stats
        // card below it. It vanishes again once the update finishes.
        if let Some(u) = &self.update {
            let h = (4.0 * ch).min(sb.h);
            let top = Rect { h, ..sb };
            crate::panecard::push_card(scenes, top, cw, ch, "UPDATE", |cols, rows| {
                crate::updatecard::update_cells(u, cols, rows)
            });
            sb = Rect {
                y: sb.y + h + GAP,
                h: (sb.h - h - GAP).max(0.0),
                ..sb
            };
        }
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
