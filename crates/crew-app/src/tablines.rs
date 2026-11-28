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

    /// Every line of the PANES list, in drawing order — a card's two lines
    /// and the gaps between cards when the panel has the room ([`Self::pane_cards`]).
    pub(crate) fn nav_lines(&self) -> Vec<Line> {
        let cards = self.pane_cards();
        let mut out = Vec::new();
        if self.tabs.list.len() < 2 {
            crate::navpanecard::push_entries(&mut out, 0..self.panes.len(), cards);
            return out;
        }
        for t in &self.tabs.list {
            out.push(Line::Tab(t.id));
            let mine = (0..self.panes.len()).filter(|&i| self.tab_of(i) == t.id);
            crate::navpanecard::push_entries(&mut out, mine, cards);
            out.push(Line::Edge(t.id));
        }
        out
    }

    /// Whether the PANES list draws as two-line cards: the Panes section is
    /// open and the panel has a row for every card line (`navpanecard::fits`).
    /// Read from the panel's height alone — the layout itself counts
    /// [`Self::nav_lines`], so asking it would ask this.
    pub(crate) fn pane_cards(&self) -> bool {
        if !self.config.show_nav || self.config.nav_collapsed {
            return false;
        }
        if self.nav_section() != crate::navsection::NavSection::Panes || self.panes.is_empty() {
            return false;
        }
        let Some((cw, ch, _sw, sh, scale)) = self.frame_geometry() else {
            return false;
        };
        let (nav, g) = (self.nav_px(scale), self.gutter());
        let sb = crate::chrome::stats_card_rect(sh, nav, g, ch, self.nav_top_card());
        let (_, rows) = crate::layout::card_inner_cells(sb.w, sb.h, cw, ch);
        let tabs = self.tabs.list.len();
        let groups = match tabs {
            0 | 1 => 1,
            _ => (self.tabs.list.iter())
                .filter(|t| (0..self.panes.len()).any(|i| self.tab_of(i) == t.id))
                .count(),
        };
        let compact = self.panes.len() + if tabs < 2 { 0 } else { 2 * tabs };
        crate::navpanecard::fits(rows, compact, self.panes.len(), groups)
    }

    /// The hover card's view of pane `i` under the pointer: the rows it is
    /// drawn on, the text drawn there, and the text whole. A card is two rows —
    /// its name and what it is doing — and either may be the one cut.
    pub(crate) fn pane_tip(&self, i: usize) -> Option<(Vec<u16>, Vec<String>, Vec<String>)> {
        let p = self.panes.get(i)?;
        let (rel, l) = self.sidebar_row()?;
        // `sidebar_row` counts the card's border; the panel's rows start under it.
        let row = rel.checked_sub(1)?;
        if !self.pane_cards() {
            let title = p.title_text();
            return Some((vec![row], vec![title.clone()], vec![title]));
        }
        let k = crate::hit::sidebar_pane_index(rel, l.panes_top?)?;
        let top = match self.nav_lines().get(k)? {
            Line::Detail(_) => row.checked_sub(1)?,
            _ => row,
        };
        let name = crate::panestate::name_of(p);
        let detail = crate::panestate::detail_of(p).unwrap_or_default();
        let said = crate::panegist::split_time(&detail).0.to_string();
        Some((
            vec![top, top + 1],
            vec![name, said],
            vec![p.title_text(), detail],
        ))
    }

    /// The rail's lines: the list's, without the cards' bottom edges (the
    /// rail is never in card density, so it has no second lines or gaps).
    pub(crate) fn rail_lines(&self) -> Vec<Line> {
        let mut l = self.nav_lines();
        l.retain(|l| matches!(l, Line::Pane(_) | Line::Tab(_)));
        l
    }

    /// The pane on line `k` of the open nav's list, if a pane is there: either
    /// line of its card.
    pub(crate) fn nav_pane_at(&self, k: usize) -> Option<usize> {
        match self.nav_lines().get(k)? {
            Line::Pane(i) | Line::Detail(i) => Some(*i),
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
                let k = crate::hit::sidebar_pane_index(rel_row, l.panes_top?)?;
                *self.nav_lines().get(k)?
            }
        };
        match line {
            Line::Tab(id) | Line::Edge(id) => Some(id),
            Line::Pane(_) | Line::Detail(_) | Line::Gap => None,
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

/// Cmd+Opt+1…9: the tab number it names. Off a Mac this is never one:
/// Ctrl+1…9 is [`crate::winnav`]'s.
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
