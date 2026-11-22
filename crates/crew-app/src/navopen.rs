//! The open nav as one card: the activity bar down its left edge, a rule, and the open
//! section's panel beside it (see [`crate::navbar`], [`crate::navpanel`]).
//!
//! One card rather than two, because the nav is one panel (on a relief theme it is moulded from
//! the page as one); the bar and the panel are parted inside it by a rule, as its sections were.
//!
//! The bar's icons answer clicks here, ahead of every row the panel answers for
//! (`btnpress::docked_press`). The panel's hit paths are unchanged: they are row-based, and
//! [`CrewApp::nav_hit_geometry`] hands them the panel's rect — the card less the bar — so a
//! click on the bar never reaches them.
use crew_render::PaneScene;

use crate::app::CrewApp;
use crate::chrome;
use crate::layout::Rect;
use crate::navbar::{BarHit, BAR_COLS};
use crate::navsection::NavSection;

/// Columns the bar and its rule take from the card's interior.
pub(crate) const BAR_TAKES: u16 = BAR_COLS + 1;

/// The panel's part of the card `sb`: the card less the bar and its rule, at the same height,
/// so its rows are the card's rows.
pub(crate) fn panel_rect(sb: Rect, cw: f32) -> Rect {
    let dx = (f32::from(BAR_TAKES) * cw).min(sb.w);
    Rect {
        x: sb.x + dx,
        w: sb.w - dx,
        ..sb
    }
}

impl CrewApp {
    /// The section the panel shows.
    pub(crate) fn nav_section(&self) -> NavSection {
        self.config.nav_section()
    }

    /// The activity bar's width and its rule, in LOGICAL px — the unit `nav_width` is kept in.
    /// Measured in cells, as the rail is: what it holds must still fit at 24 pt.
    pub(crate) fn nav_bar_w(&self) -> f32 {
        match self.frame_geometry() {
            Some((cw, _, _, _, scale)) => f32::from(BAR_TAKES) * cw / scale.max(0.1),
            None => crate::navrail::FALLBACK_W,
        }
    }

    /// The badges the bar carries this frame.
    pub(crate) fn nav_badges(&self, rows: &[crate::panelist::PaneRow]) -> crate::navbar::Badges {
        let spin = crate::update::spinner_frame(crate::anim::now_ms());
        crate::navbar::Badges {
            panes: crate::navbar::panes_badge(rows, spin),
            // What waits on YOU: a stopped prompt or a plan. Tasks in flight are not, and
            // the card's "all quiet" row is not anything.
            waiting: self.glance().map_or(0, |g| {
                use crate::navglance::Wait;
                g.waiting
                    .iter()
                    .filter(|w| matches!(w.kind, Wait::Blocked | Wait::Plan))
                    .count()
            }),
        }
    }

    /// The open card's cells and paint: bar, rule, then the section at the panel's columns.
    pub(crate) fn push_sections(
        &self,
        scenes: &mut Vec<PaneScene>,
        sb: Rect,
        (cw, ch): (f32, f32),
        (legend, legend_fg): (&str, (u8, u8, u8)),
    ) {
        let list = self.nav_list();
        let rows_flat = self.pane_rows();
        let badges = self.nav_badges(&rows_flat);
        let glance = self.glance();
        let weather = crate::navweather::now().map(|w| crate::navweather::line(&w));
        let section = self.nav_section();
        let hover = self.bar_at_cursor();
        let input = crate::navpanel::PanelInput {
            panes: &list,
            log: &self.log,
            log_back: self.log_back,
            glance: glance.as_ref(),
            strip: weather.as_deref(),
        };
        let sidebar = &self.sidebar;
        // `ch / cw` goes with the paint so a chart's circles stay round.
        let aspect = ch / cw;
        crate::panelcard::push_card_art(scenes, sb, cw, ch, legend, legend_fg, |cols, rows| {
            let pcols = cols.saturating_sub(BAR_TAKES);
            let l = self.nav_layout(section, rows);
            let mut cells = crate::navbar::bar_cells(Some(section), badges, rows);
            cells.extend(crate::navbar::divider_cells(BAR_COLS, rows));
            let panel = match section {
                NavSection::Explorer => self.explorer_cells(pcols, rows),
                _ => sidebar.panel_cells(section, &l, pcols, rows, &input),
            };
            for mut c in panel {
                c.col += BAR_TAKES;
                cells.push(c);
            }
            let mut paint = crate::navicons::bar_paint(Some(section), hover, rows, aspect);
            paint.extend(
                sidebar
                    .panel_paint(section, &l, (pcols, rows), aspect, glance.as_ref())
                    .into_iter()
                    .map(|p| p.shifted(f32::from(BAR_TAKES), 0.0)),
            );
            (cells, paint)
        });
    }

    /// The panel's division for `section` at `rows` content rows — what the draw, the paint and
    /// the hit paths all read.
    pub(crate) fn nav_layout(&self, section: NavSection, rows: u16) -> crate::navlayout::NavLayout {
        crate::navsectionlayout::layout_for(
            section,
            rows,
            self.nav_tail(),
            self.log.len(),
            self.nav_lines().len(),
        )
    }

    /// The bar's icon under the cursor, open or collapsed.
    pub(crate) fn bar_at_cursor(&self) -> Option<BarHit> {
        if !self.config.show_nav {
            return None;
        }
        let (cw, ch, _sw, sh, scale) = self.frame_geometry()?;
        let collapsed = self.config.nav_collapsed;
        let sb = chrome::stats_card_rect(
            sh,
            self.nav_px(scale),
            self.gutter(),
            ch,
            self.nav_top_card(),
        );
        // The bar is the card's first `BAR_COLS` interior columns, past its border column.
        let bar_end = sb.x + f32::from(1 + BAR_COLS) * cw;
        let (x, y) = self.cursor;
        if !chrome::point_in(sb, x, y) || x >= bar_end {
            return None;
        }
        let (_, rows) = crate::layout::card_inner_cells(sb.w, sb.h, cw, ch);
        // The collapsed card is drawn level with the panes (`panellevel`); the open one is not.
        let dy = match collapsed {
            true => crate::panellevel::level_dy(sb, &self.panes, cw, ch),
            false => 0.0,
        };
        let row = ((y - sb.y - dy) / ch).floor() as i32 - 1;
        let rows = crate::navrailbar::visible_rows(rows, dy);
        crate::navbar::at_row(u16::try_from(row).ok()?, rows)
    }

    /// A press on the bar: an icon opens its section — or, already open, puts the panel away,
    /// as VS Code's do — and the gear opens the settings. `true` when it was the bar's.
    pub(crate) fn nav_bar_press(&mut self) -> bool {
        match self.bar_at_cursor() {
            Some(BarHit::Section(s)) => self.pick_nav_section(s),
            Some(BarHit::Settings) => self.spawn_settings_pane(),
            None => return false,
        }
        self.redraw();
        true
    }

    /// Open section `s`: switch to it, opening the panel if it was put away; or, when it is the
    /// one already showing, put the panel away.
    pub(crate) fn pick_nav_section(&mut self, s: NavSection) {
        let open = !self.config.nav_collapsed;
        if open && self.nav_section() == s {
            self.toggle_nav_collapsed();
            return;
        }
        self.config.nav_section = s.as_str().to_string();
        if !open {
            self.toggle_nav_collapsed();
        }
        self.config.save();
    }
}

#[cfg(test)]
#[path = "navopen_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "navopentabs_tests.rs"]
mod tab_tests;

#[cfg(test)]
#[path = "navopenfix_tests.rs"]
mod fix;
