//! The nav's PANES list once there is more than one tab: a small card per
//! tab — `╭─ 2 · web ──── 3 ─╮` — with its panes inside, the tab on screen
//! framed in the accent and the rest in the quiet border (a tab with a pane
//! that needs you takes the bell's colour, so it can be seen from any other).
//!
//! With one tab the list stays exactly what it was: a card around the whole
//! crew says nothing the PANES rule does not already say.
//!
//! [`CrewApp::nav_lines`] is the single answer to "what is on line k" — the
//! drawing, the layout's row count and every hit test read it, so a click can
//! never land on a different row from the one that lit up under it.
use crew_render::CellView;

use crate::panelist::{pane_cells, PaneRow};

/// What one line of the list stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Line {
    /// The pane at this index.
    Pane(usize),
    /// A tab card's top edge, which carries its number and name.
    Tab(u32),
    /// A tab card's bottom edge.
    Edge(u32),
}

/// One tab's card: its legend and its panes' rows.
pub(crate) struct TabBlock {
    pub num: usize,
    pub title: String,
    pub active: bool,
    pub hovered: bool,
    pub rows: Vec<PaneRow>,
}

impl TabBlock {
    /// A pane in this tab is asking for you: the card (and the rail's
    /// divider) takes the bell's colour when the tab is not on screen.
    pub(crate) fn needs_you(&self) -> bool {
        self.rows.iter().any(|r| r.attention.is_some())
    }
}

/// The PANES list, as the nav draws it.
pub(crate) enum NavList {
    Flat(Vec<PaneRow>),
    Tabs(Vec<TabBlock>),
}

impl From<Vec<PaneRow>> for NavList {
    fn from(rows: Vec<PaneRow>) -> Self {
        NavList::Flat(rows)
    }
}

impl NavList {
    /// Lines below the PANES rule.
    pub(crate) fn len(&self) -> usize {
        match self {
            NavList::Flat(r) => r.len(),
            NavList::Tabs(b) => b.iter().map(|b| b.rows.len() + 2).sum(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The PANES section: its rule on row 0 and up to `limit` lines under it.
    pub(crate) fn cells(&self, cols: u16, limit: usize, spin: char) -> Vec<CellView> {
        let blocks = match self {
            NavList::Flat(rows) => return pane_cells(rows, cols, limit, spin),
            NavList::Tabs(b) => b,
        };
        let t = crew_theme::theme();
        let total: usize = blocks.iter().map(|b| b.rows.len()).sum();
        // `shown/total` when a short nav cuts the list, as the flat list's
        // key says, so a hidden pane is never counted as on view.
        let shown = shown(blocks, limit);
        let key = format!(
            "{} tabs \u{b7} {}",
            blocks.len(),
            crate::navtext::depth_key(shown, total)
        );
        let (accent, muted) = (crate::palette::accent(), t.text_muted);
        let mut out = crate::boxdraw::section_header_key(
            "PANES",
            &key,
            cols,
            t.border_normal,
            accent,
            muted,
            t.page_bg,
        );
        let mut row = 1u16;
        let end = 1 + limit.min(usize::from(u16::MAX - 1)) as u16;
        for b in blocks {
            let frame = match (b.active, b.needs_you()) {
                (true, _) => accent,
                (false, true) => t.bell,
                (false, false) => t.border_normal,
            };
            if row < end {
                out.extend(crate::tabcard::top(b, cols, row, frame));
                row += 1;
            }
            for r in &b.rows {
                if row >= end {
                    break;
                }
                // The row as the flat list draws it, three columns narrower
                // and one in, so the card's sides keep a column of air.
                let one = pane_cells(std::slice::from_ref(r), cols.saturating_sub(3), 1, spin);
                out.extend(one.into_iter().filter(|c| c.row == 1).map(|mut c| {
                    c.col += 1;
                    c.row = row;
                    c
                }));
                out.extend(crate::tabcard::sides(cols, row, frame));
                row += 1;
            }
            if row < end {
                out.extend(crate::tabcard::bottom(cols, row, frame));
                row += 1;
            }
        }
        out
    }

    /// The collapsed rail's rows: a `─N─` divider per tab, then its panes.
    pub(crate) fn rail_cells(&self, cols: u16, rows: u16, spin: char) -> Vec<CellView> {
        let blocks = match self {
            NavList::Flat(r) => return crate::navrail::rail_cells(r, cols, rows, spin),
            NavList::Tabs(b) => b,
        };
        let mut out = Vec::new();
        let mut row = 0u16;
        for b in blocks {
            if row >= rows {
                break;
            }
            out.extend(crate::tabcard::divider(b, cols, row));
            row += 1;
            let room = rows.saturating_sub(row);
            out.extend(
                crate::navrail::rail_cells(&b.rows, cols, room, spin)
                    .into_iter()
                    .map(|mut c| {
                        c.row += row;
                        c
                    }),
            );
            row += (b.rows.len() as u16).min(room);
        }
        out
    }

    /// Rows the rail uses: every pane, plus one divider per tab.
    pub(crate) fn rail_len(&self) -> usize {
        match self {
            NavList::Flat(r) => r.len(),
            NavList::Tabs(b) => b.iter().map(|b| b.rows.len() + 1).sum(),
        }
    }
}

/// Panes whose rows land within the first `limit` lines: each card spends a
/// line on its top edge before its rows.
fn shown(blocks: &[TabBlock], limit: usize) -> usize {
    let mut line = 0;
    let mut n = 0;
    for b in blocks {
        line += 1;
        n += b.rows.len().min(limit.saturating_sub(line));
        line += b.rows.len() + 1;
    }
    n
}

#[cfg(test)]
#[path = "tabnav_tests.rs"]
mod tests;
