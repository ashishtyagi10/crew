//! The panel beside the activity bar: whichever section is open, drawn at the rows its
//! [`NavLayout`] gave it (see [`crate::navsectionlayout`]). One entry for the cells and one for
//! the paint layer, both handed the SAME layout the hit paths read this frame.
use crew_render::{CellView, Paint};

use crate::applog::LogEntry;
use crate::navglance::Glance;
use crate::navlayout::NavLayout;
use crate::navsection::NavSection;
use crate::statspane::StatsPane;

/// What a section's cells are drawn from — the frame's readings, gathered once.
pub(crate) struct PanelInput<'a> {
    pub panes: &'a crate::tabnav::NavList,
    pub log: &'a [LogEntry],
    /// How far back the LOG is scrolled — 0 follows the newest line.
    pub log_back: usize,
    /// The glance cards (`None`: `/nav log`, the LOG alone).
    pub glance: Option<&'a Glance>,
    /// The weather strip under the clock.
    pub strip: Option<&'a str>,
}

impl StatsPane {
    /// The open section's cells, in the panel's own coordinates.
    pub(crate) fn panel_cells(
        &self,
        section: NavSection,
        l: &NavLayout,
        cols: u16,
        rows: u16,
        input: &PanelInput,
    ) -> Vec<CellView> {
        match section {
            NavSection::System => self.system_cells(cols, rows, input.strip),
            NavSection::Activity => activity_cells(l, cols, input),
            NavSection::Panes => panes_cells(l, cols, rows, input.panes),
            NavSection::Explorer => explorer_cells(cols),
            NavSection::Inbox => crate::navinbox::cells(
                &crate::inboxglance::state(),
                &crate::inboxglance::unread(),
                None,
                crate::chattime::unix_now_ms() as i64,
                (cols, rows),
            ),
            NavSection::Far => crate::navfar::cells(&crate::farrecent::list(), None, (cols, rows)),
        }
    }

    /// The open section's sub-cell paint: the System section's charts and meters, the Activity
    /// cards' curve and meters, nothing for the rest.
    pub(crate) fn panel_paint(
        &self,
        section: NavSection,
        l: &NavLayout,
        (cols, rows): (u16, u16),
        aspect: f32,
        glance: Option<&Glance>,
    ) -> Vec<Paint> {
        match (section, glance) {
            (NavSection::System, _) => self.system_paint(cols, rows, aspect),
            (NavSection::Activity, Some(g)) => crate::navslot::slot_paint(g, l, cols, aspect),
            _ => Vec::new(),
        }
    }
}

/// The glance cards, then the LOG under them (or the LOG alone under `/nav log`).
fn activity_cells(l: &NavLayout, cols: u16, input: &PanelInput) -> Vec<CellView> {
    let mut out = crate::navslot::slot_cells(input.glance, input.log, input.log_back, l, cols);
    if input.glance.is_some() && l.log_lines > 0 {
        for mut c in crate::navlog::log_cells(input.log, cols, l.log_lines, input.log_back) {
            c.row += l.log_top;
            out.push(c);
        }
    }
    out
}

/// The tabs and the pane list, from the panel's top.
fn panes_cells(
    l: &NavLayout,
    cols: u16,
    rows: u16,
    panes: &crate::tabnav::NavList,
) -> Vec<CellView> {
    let Some(top) = l.panes_top else {
        return Vec::new();
    };
    // Header + one row per pane, as far as the panel goes.
    let limit = rows.saturating_sub(top + 1) as usize;
    // A busy pane's row spins; with motion off it holds one frame.
    let spin = crate::update::spinner_frame(crate::anim::now_ms());
    let mut out = panes.cells(cols, limit, spin);
    for c in &mut out {
        c.row += top;
    }
    out
}

/// The Explorer's rule alone: its tree is drawn by the app, which knows the root
/// (`explorerhit::explorer_cells`).
fn explorer_cells(cols: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let head = crate::palette::accent();
    crate::boxdraw::section_header("EXPLORER", cols, t.border_normal, head, t.page_bg)
}

/// Every section's cells at once, for the audits that hold all the nav draws to a contrast or
/// glass floor — they ask "is any of it unreadable", not "where is it".
#[cfg(test)]
impl StatsPane {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cells(
        &self,
        cols: u16,
        rows: u16,
        panes: &crate::tabnav::NavList,
        log: &[LogEntry],
        log_back: usize,
        glance: Option<&Glance>,
        strip: Option<&str>,
    ) -> Vec<CellView> {
        let input = PanelInput {
            panes,
            log,
            log_back,
            glance,
            strip,
        };
        let tail = crate::navslot::tail(glance, log.len());
        NavSection::ALL
            .into_iter()
            .flat_map(|s| {
                let l = crate::navsectionlayout::layout_for(s, rows, tail, log.len(), panes.len());
                self.panel_cells(s, &l, cols, rows, &input)
            })
            .collect()
    }

    /// Every section's paint at once — the audits' counterpart of [`Self::cells`].
    pub(crate) fn chart_paint(
        &self,
        cols: u16,
        rows: u16,
        aspect: f32,
        glance: Option<&Glance>,
        panes: usize,
    ) -> Vec<Paint> {
        let tail = crate::navslot::tail(glance, 0);
        NavSection::ALL
            .into_iter()
            .flat_map(|s| {
                let l = crate::navsectionlayout::layout_for(s, rows, tail, 0, panes);
                self.panel_paint(s, &l, (cols, rows), aspect, glance)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "navpanel_tests.rs"]
mod tests;
