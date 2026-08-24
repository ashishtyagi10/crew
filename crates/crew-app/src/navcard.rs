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
        // While a LOUD `/update` runs, dock a distinct UPDATE card on top of the
        // stats card, shrinking the stats card below it (chrome::stats_card_rect —
        // the same rect the PANES hit-test uses). A silent background run (see
        // `crate::autoupdate`) stays invisible — no card, no reserved space —
        // until a manual `/update` upgrades it to loud.
        let loud = self.update.as_ref().filter(|u| !u.silent);
        if let Some(u) = loud {
            let top = Rect {
                h: (chrome::UPDATE_CARD_ROWS * ch).min(full.h),
                ..full
            };
            crate::panelcard::push_card(scenes, top, cw, ch, "UPDATE", |cols, rows| {
                crate::updatecard::update_cells(u, cols, rows)
            });
        }
        let sb = chrome::stats_card_rect(sh, self.nav_px(scale), GAP, ch, loud.is_some());
        let pane_rows = self.pane_rows();
        let sidebar = &self.sidebar;
        let log = &self.log;
        let log_back = self.log_back;
        let (legend, legend_fg) = match &self.parked_update {
            Some((v, at)) => (
                crate::restartnote::legend(v, title_max_cols(sb, cw, ch)),
                crate::restartnote::legend_fg(crate::anim::now_ms(), *at),
            ),
            None => (
                concat!("crew v", env!("CARGO_PKG_VERSION")).to_string(),
                crew_theme::theme().legend_off,
            ),
        };
        crate::panelcard::push_card_titled(scenes, sb, cw, ch, &legend, legend_fg, |cols, rows| {
            sidebar.cells(cols, rows, &pane_rows, log, log_back)
        });
    }

    /// One row per open pane for the sidebar PANES list. A row carries the
    /// `[+]` restore marker whenever its pane is NOT visible in the content
    /// area — minimized into the nav, covered while another pane is zoomed,
    /// or standing behind the strip's `+N` overflow tile — so the list always
    /// says which panes are actually on screen. Clicking (or Cmd+N-focusing)
    /// such a row brings the pane back either way.
    /// The focused pane's name, for the input bar's bottom-border legend. The
    /// same `title_text()` the PANES list and the pane's own card legend show,
    /// so one pane is never called two things on one screen. `None` when there
    /// are no panes at all (the welcome screen), which is the one moment the
    /// question has no answer.
    pub(crate) fn focused_pane_name(&self) -> Option<String> {
        self.panes.get(self.focused).map(|p| p.title_text())
    }

    pub(crate) fn pane_rows(&self) -> Vec<crate::panelist::PaneRow> {
        // Zoom draws only the focused pane (clamped like build_frame clamps).
        let zoomed_on = self.focused.min(self.panes.len().saturating_sub(1));
        // Panes standing behind the strip's `+N` overflow tile have no
        // thumbnail — not visible anywhere in the content area — so their
        // rows get the [+] marker too. Same placement derivation the frame
        // draws from; before the renderer reports geometry the set is empty.
        let strip_hidden = self
            .placed_grid()
            .map(|(_, placed)| placed.strip_hidden(self.grid.minimized()))
            .unwrap_or_default();
        // One clock read per frame keeps every row's blink phase in step.
        let now = crate::anim::now_ms();
        // One hit-test per frame, not one per row.
        let hovered = self.pane_at_sidebar();
        self.panes
            .iter()
            .enumerate()
            .map(|(i, p)| crate::panelist::PaneRow {
                index: i + 1,
                title: p.title_text(),
                focused: i == self.focused,
                activity: p.activity,
                minimized: p.hidden || (self.zoomed && i != zoomed_on) || strip_hidden.contains(&i),
                attention: p.attention.map(|a| (a.glyph(), a.visible(now))),
                busy: crate::paneview::pane_busy(p),
                hovered: hovered == Some(i),
            })
            .collect()
    }
}

/// Available title column budget for the parked-update legend: the card's
/// total column count is `card_inner_cells(rect, cw, ch).0 + 2` (the two
/// border columns), and [`crate::boxdraw::title_budget`] is the single
/// authority on how many of those a legend may use before it ellipsizes —
/// pre-fitting keeps the restart note's version-preserving shortening
/// ahead of the generic `…` clip.
fn title_max_cols(rect: Rect, cw: f32, ch: f32) -> usize {
    let (icols, _) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);
    crate::boxdraw::title_budget(icols + 2)
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;
    use crate::farpane::FarPane;
    use crate::layout::Rect;
    use crate::pane::{Pane, PaneContent};
    use crew_term::GridSize;

    #[test]
    fn title_max_cols_is_exactly_titled_cards_truncation_budget() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        let (cw, ch) = (8.0f32, 16.0f32);
        let max = super::title_max_cols(rect, cw, ch);
        let (icols, irows) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);

        // A title exactly `max` chars long survives untruncated.
        let fits: String = "x".repeat(max);
        let cells = crate::boxdraw::titled_card(
            icols + 2,
            irows + 2,
            &fits,
            (0, 0, 0),
            (0, 0, 0),
            (0, 0, 0),
        );
        assert_eq!(cells.iter().filter(|c| c.c == 'x').count(), max);

        // One char more than `max` gets ellipsized back down to `max` total
        // columns: `max - 1` kept chars plus the `…` marker.
        let overflows: String = "x".repeat(max + 1);
        let cells2 = crate::boxdraw::titled_card(
            icols + 2,
            irows + 2,
            &overflows,
            (0, 0, 0),
            (0, 0, 0),
            (0, 0, 0),
        );
        assert_eq!(cells2.iter().filter(|c| c.c == 'x').count(), max - 1);
        assert_eq!(cells2.iter().filter(|c| c.c == '…').count(), 1);
    }

    fn far_pane(name: &str) -> Pane {
        Pane {
            content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            name: Some(name.to_string()),
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        }
    }

    #[test]
    fn zoom_marks_every_covered_pane_restorable() {
        let mut app = CrewApp::default();
        for n in ["a", "b", "c"] {
            app.panes.push(far_pane(n));
        }
        app.focused = 1;
        app.zoomed = true;
        let rows = app.pane_rows();
        assert!(!rows[1].minimized, "the zoomed pane is on screen");
        assert!(
            rows[0].minimized && rows[2].minimized,
            "panes covered by the zoom get the [+] marker"
        );
    }

    #[test]
    fn attention_reaches_the_pane_row_as_a_glyph() {
        let mut app = CrewApp::default();
        for n in ["a", "b"] {
            app.panes.push(far_pane(n));
        }
        crate::attention::raise(
            &mut app.panes[1],
            crate::notify::NotifyKind::Bell,
            crate::anim::now_ms(),
        );
        let rows = app.pane_rows();
        assert_eq!(rows[0].attention, None);
        // Fresh marker: bell glyph, blink phase starts visible.
        assert_eq!(rows[1].attention, Some(('!', true)));
    }

    #[test]
    fn grid_marks_only_nav_hidden_panes_restorable() {
        let mut app = CrewApp::default();
        for n in ["a", "b", "c"] {
            app.panes.push(far_pane(n));
        }
        app.panes[2].hidden = true;
        let rows = app.pane_rows();
        assert!(!rows[0].minimized && !rows[1].minimized);
        assert!(rows[2].minimized);
    }
}
