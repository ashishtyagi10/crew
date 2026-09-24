//! The nav at its smallest: one column of panes and the chevron that opens it.
//!
//! The full nav is a dashboard — clock, dials, load, host, net, git, the LOG or
//! the glance cards, then the pane list. It is worth its 160-320 px when you
//! are reading it and it is a third of a laptop screen when you are not, and
//! what you actually need from it most of the time is the last section: which
//! panes exist, which one has focus, and which one wants you.
//!
//! So that is the rail, and nothing else. Seven columns: the focus caret, the
//! pane's number, and the same mark its full row would carry — attention over
//! busy over unread, in the same three colours, so the two readings can never
//! disagree about what a pane is doing. Everything else the nav knows is one
//! click away and stays there.
//!
//! The chevron is the first character of the card's own legend, which is where
//! this window already puts what a card is called: `› ` collapsed, `‹ crew
//! v0.22.41` open. One place in both states, and it points the way the edge
//! will move.
use crew_render::{CellView, PaneScene};

use crate::app::CrewApp;
use crate::chrome;
use crate::layout::Rect;
use crate::palette::accent;
use crate::panelist::PaneRow;

/// Columns the rail's card occupies, its two borders included. Five inside:
/// caret, two digits of pane number, a column of air, and the mark.
pub(crate) const RAIL_COLS: u16 = 7;

/// Logical width to fall back on before the renderer has reported a cell
/// size — the rail is measured in CELLS, since what it holds is a little
/// fixed table that must still hold at 24 pt.
pub(crate) const FALLBACK_W: f32 = 56.0;

/// Where the chevron lands: a card's legend opens at column 3 — corner, rule,
/// space, then the first character of the title (`boxdraw::section_header`).
pub(crate) const CHEVRON_COL: u16 = 3;
/// Cells its click target spans, centred on it: the space before, the
/// chevron, the space after. A one-cell target is one you have to aim at.
pub(crate) const CHEVRON_COLS: u16 = 3;

/// Points the way the edge will move: open it, or put it away.
pub(crate) fn chevron(collapsed: bool) -> char {
    match collapsed {
        true => '\u{203a}',
        false => '\u{2039}',
    }
}

/// The card's legend. Collapsed there is room for the chevron and nothing
/// else, which is the whole point of it.
pub(crate) fn legend(collapsed: bool, full: &str) -> String {
    match collapsed {
        true => chevron(true).to_string(),
        false => format!("{} {full}", chevron(false)),
    }
}

/// The rail's width in physical px for a cell `cw` wide. One pixel of slack
/// past the seventh column: the card's own column count is `floor(w / cw)`,
/// and landing exactly on the boundary is one float rounding away from a
/// six-column rail with nowhere to put the mark.
pub(crate) fn px(cw: f32) -> f32 {
    f32::from(RAIL_COLS) * cw + 1.0
}

/// The chevron's click target on a card at `card`.
pub(crate) fn chevron_rect(card: Rect, cw: f32, ch: f32) -> Rect {
    Rect {
        x: card.x + f32::from(CHEVRON_COL - 1) * cw,
        y: card.y,
        w: f32::from(CHEVRON_COLS) * cw,
        h: ch,
    }
}

/// Which pane the rail's `rel_row` (rows from the card's top edge, so 0 is
/// its border) stands for. `None` off the list.
pub(crate) fn pane_at_row(rel_row: u16, rows: u16, panes: usize) -> Option<usize> {
    let i = rel_row.checked_sub(1)?;
    (i < rows && usize::from(i) < panes).then_some(usize::from(i))
}

/// One row per pane: caret, number, mark. `spin` is the busy frame the full
/// list uses, so a pane that is working turns the same way in both.
pub(crate) fn rail_cells(panes: &[PaneRow], cols: u16, rows: u16, spin: char) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    for (row, p) in panes.iter().take(usize::from(rows)).enumerate() {
        let row = row as u16;
        let head = format!("{}{}", if p.focused { '\u{25b8}' } else { ' ' }, p.index);
        let head_fg = match p.focused || p.hovered {
            true => accent(),
            false => t.text_muted,
        };
        // Two columns of air held back for the mark: the number is what gets
        // cut if a crew ever runs to four digits, never the thing that says
        // the pane needs you.
        let budget = cols.saturating_sub(2);
        crate::navtext::put_at(&mut out, &head, 0, row, budget, head_fg);
        // The mark keeps the full list's priority and colours: a raised
        // attention wins, then work in progress, then lines not yet read.
        let mark = match (p.attention, p.busy, p.activity || p.unread > 0) {
            (Some((glyph, on)), _, _) => on.then_some((glyph, t.bell)),
            (None, true, _) => Some((spin, accent())),
            (None, false, true) => Some(('\u{25cf}', t.activity)),
            _ => None,
        };
        if let Some((glyph, fg)) = mark {
            let at = cols.saturating_sub(1);
            crate::navtext::put_at(&mut out, &glyph.to_string(), at, row, cols, fg);
        }
    }
    out
}

impl CrewApp {
    /// The rail's row under the cursor. Its own mapping, because the rail has
    /// no NavLayout to divide: row 1 of the card is the first pane.
    pub(crate) fn pane_at_rail(&self) -> Option<usize> {
        let (cw, ch, _sw, sh, scale) = self.frame_geometry()?;
        let sb = chrome::stats_card_rect(sh, self.nav_px(scale), self.gutter(), ch, false);
        if !chrome::point_in(sb, self.cursor.0, self.cursor.1) {
            return None;
        }
        let (_, rows) = crate::layout::card_inner_cells(sb.w, sb.h, cw, ch);
        let rel = ((self.cursor.1 - sb.y) / ch).floor() as u16;
        crate::navrail::pane_at_row(rel, rows, self.panes.len())
    }

    /// Whether the cursor is on the nav card's chevron — the one control that
    /// is in both states, on the card's own legend row.
    pub(crate) fn nav_chevron_at_cursor(&self) -> bool {
        if !self.config.show_nav {
            return false;
        }
        let Some((cw, ch, _sw, sh, scale)) = self.frame_geometry() else {
            return false;
        };
        let sb = chrome::stats_card_rect(
            sh,
            self.nav_px(scale),
            self.gutter(),
            ch,
            self.nav_top_card(),
        );
        let hit = crate::navrail::chevron_rect(sb, cw, ch);
        chrome::point_in(hit, self.cursor.0, self.cursor.1)
    }

    /// Push the collapsed nav: the pane list at the top, the readings that
    /// survive five columns along the bottom (`navrailfoot`). It takes the
    /// column's full height — the UPDATE and RESTART cards want words and have
    /// nowhere to put them here, so they wait for the nav to be opened
    /// (`nav_top_card`).
    pub(crate) fn push_rail(
        &self,
        scenes: &mut Vec<PaneScene>,
        sh: f32,
        scale: f32,
        cw: f32,
        ch: f32,
    ) {
        let sb = chrome::stats_card_rect(sh, self.nav_px(scale), self.gutter(), ch, false);
        let rows = self.pane_rows();
        let spin = crate::update::spinner_frame(crate::anim::now_ms());
        let legend = legend(true, "");
        let foot = crate::navrailfoot::Foot {
            time: crate::clock::now_strings().0.chars().take(5).collect(),
            sky: crate::navweather::now().map(|w| crate::navrailrow::sky(&w)),
            stats: self.sidebar.stats(),
            git: self.sidebar.git_info().cloned(),
        };
        // `ch / cw` goes with the meters so a capsule stays a capsule when the
        // font changes (see `crate::plot::Canvas`).
        let aspect = ch / cw;
        let fg = crew_theme::theme().legend_off;
        crate::panelcard::push_card_art(scenes, sb, cw, ch, &legend, fg, |cols, irows| {
            let mut cells = rail_cells(&rows, cols, irows, spin);
            let used = rows.len().min(usize::from(irows)) as u16;
            let (foot, paint) = crate::navrailfoot::foot(&foot, cols, irows, used, aspect);
            cells.extend(foot);
            (cells, paint)
        });
    }
}

#[cfg(test)]
#[path = "navrail_tests.rs"]
mod tests;
