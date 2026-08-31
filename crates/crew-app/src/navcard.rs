//! Builds the left-nav sidebar PaneScene: the StatsPane sections (clock, system,
//! load, host, net, git, LOG) plus the live pane list, framed by a fieldset card
//! whose legend carries the running version — so the build is always visible in
//! the left nav (replacing the old `/about` status flash).
use crew_render::PaneScene;

use crate::app::{gap, CrewApp};
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
        let full = chrome::sidebar_rect(sh, self.nav_px(scale), gap());
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
        let sb = chrome::stats_card_rect(sh, self.nav_px(scale), gap(), ch, loud.is_some());
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
        // The card carries two layers: the cells, and the sub-cell paint its
        // charts are drawn on. `ch / cw` goes with them so a chart's circles
        // stay round and its proportions survive a font change.
        let aspect = ch / cw;
        crate::panelcard::push_card_art(scenes, sb, cw, ch, &legend, legend_fg, |cols, rows| {
            (
                sidebar.cells(cols, rows, &pane_rows, log, log_back),
                sidebar.chart_paint(cols, rows, aspect),
            )
        });
    }

    /// The docked nav's rect and its row division for the frame on screen —
    /// the one place a hit path turns a cursor position into a nav row. `None`
    /// when the nav is hidden or the renderer has not reported geometry yet.
    ///
    /// The rect must be the same one `push_sidebar` draws into, LOUD-update
    /// shift included: a silent background run draws no card, so counting it
    /// would offset every row by the height of a card that isn't on screen.
    pub(crate) fn nav_hit_geometry(&self) -> Option<(Rect, f32, crate::navlayout::NavLayout)> {
        if !self.config.show_nav {
            return None;
        }
        let (cw, ch, _sw, sh, scale) = self.frame_geometry()?;
        let sb = chrome::stats_card_rect(
            sh,
            self.nav_px(scale),
            gap(),
            ch,
            self.update.as_ref().is_some_and(|u| !u.silent),
        );
        let (_, rows) = crate::layout::card_inner_cells(sb.w, sb.h, cw, ch);
        let l = self.sidebar.layout(rows, self.log.len(), self.panes.len());
        Some((sb, ch, l))
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
            .map(|(_, placed)| placed.strip_hidden(&self.grid.minimized()))
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
                unread: match &p.content {
                    crate::pane::PaneContent::Terminal(t) => {
                        crate::unread::count(t.pty.scrollable_lines(), t.read_at)
                    }
                    _ => 0,
                },
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
#[path = "navcard_tests.rs"]
mod tests;
