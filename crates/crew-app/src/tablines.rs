//! Which line of the nav's PANES list is which ([`crate::tabnav`]), and
//! what the pointer and the keys do with a tab: the one mapping every draw
//! and hit test reads, so a click never lands on a row other than the one lit.
use crate::app::CrewApp;
use crate::panelist::PaneRow;
use crate::tabnav::{Line, NavList, TabBlock};

impl CrewApp {
    /// The tab pane `i` is counted in: its own, or — not yet adopted — the
    /// one on screen.
    fn tab_of(&self, i: usize) -> u32 {
        match self.panes.get(i).map(|p| p.tab) {
            Some(t) if t != 0 && self.tabs.pos(t).is_some() => t,
            _ => self.tabs.active,
        }
    }

    /// Every line of the PANES list, in drawing order.
    pub(crate) fn nav_lines(&self) -> Vec<Line> {
        if self.tabs.list.len() < 2 {
            return (0..self.panes.len()).map(Line::Pane).collect();
        }
        let mut out = Vec::new();
        for t in &self.tabs.list {
            out.push(Line::Tab(t.id));
            out.extend(
                (0..self.panes.len())
                    .filter(|&i| self.tab_of(i) == t.id)
                    .map(Line::Pane),
            );
            out.push(Line::Edge(t.id));
        }
        out
    }

    /// The rail's lines: the list's, without the cards' bottom edges.
    pub(crate) fn rail_lines(&self) -> Vec<Line> {
        let mut l = self.nav_lines();
        l.retain(|l| !matches!(l, Line::Edge(_)));
        l
    }

    /// The pane on line `k` of the open nav's list, if a pane is there.
    pub(crate) fn nav_pane_at(&self, k: usize) -> Option<usize> {
        match self.nav_lines().get(k)? {
            Line::Pane(i) => Some(*i),
            _ => None,
        }
    }

    /// The rail's line at `rel` rows below its card's top edge.
    pub(crate) fn rail_line(&self, rel: u16, rows: u16) -> Option<Line> {
        let lines = self.rail_lines();
        let k = crate::navrail::pane_at_row(rel, rows, lines.len())?;
        lines.get(k).copied()
    }

    /// The PANES list for this frame, grouped by tab when there are tabs.
    pub(crate) fn nav_list(&self) -> NavList {
        let mut rows: Vec<Option<PaneRow>> = self.pane_rows().into_iter().map(Some).collect();
        if self.tabs.list.len() < 2 {
            return NavList::Flat(rows.into_iter().flatten().collect());
        }
        let hovered = self.tab_at_nav();
        let blocks = (self.tabs.list.iter().enumerate())
            .map(|(k, t)| TabBlock {
                num: k + 1,
                title: self.tab_title(t.id),
                active: t.id == self.tabs.active,
                hovered: hovered == Some(t.id),
                rows: (0..rows.len())
                    .filter(|&i| self.tab_of(i) == t.id)
                    .filter_map(|i| rows[i].take())
                    .collect(),
            })
            .collect();
        NavList::Tabs(blocks)
    }

    /// The tab whose card edge is under the cursor, open nav or rail.
    pub(crate) fn tab_at_nav(&self) -> Option<u32> {
        let line = match self.config.nav_collapsed {
            true => self.rail_line_at_cursor()?,
            false => {
                let (rel_row, l) = self.sidebar_row()?;
                let k = crate::hit::sidebar_pane_index(rel_row, l.panes_top)?;
                *self.nav_lines().get(k)?
            }
        };
        match line {
            Line::Tab(id) | Line::Edge(id) => Some(id),
            Line::Pane(_) => None,
        }
    }

    /// A click on the nav: a tab's edge brings the tab on, and then no pane
    /// is the answer; otherwise the pane whose row it was.
    pub(crate) fn nav_click(&mut self) -> Option<usize> {
        if let Some(id) = self.tab_at_nav() {
            self.activate_tab(id);
            return None;
        }
        self.pane_at_sidebar()
    }
}

/// Cmd+Opt+1…9 (Ctrl+Alt+1…9 off a Mac): the tab number it names.
pub(crate) fn tab_digit(
    event: &winit::event::KeyEvent,
    mods: winit::keyboard::ModifiersState,
) -> Option<usize> {
    use winit::keyboard::{KeyCode, PhysicalKey};
    if !event.state.is_pressed() || !crate::ctrlchord::screen_held(mods, cfg!(target_os = "macos"))
    {
        return None;
    }
    let PhysicalKey::Code(code) = event.physical_key else {
        return None;
    };
    const DIGITS: [KeyCode; 9] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    DIGITS.iter().position(|&d| d == code).map(|k| k + 1)
}
