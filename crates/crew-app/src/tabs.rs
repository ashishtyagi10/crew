//! Tabs: named groups of panes, one of them on screen at a time.
//!
//! A pane says which tab it lives in ([`crate::pane::Pane::tab`], a stable
//! id rather than a position, so tabs can come and go without renumbering the
//! panes inside them). Only the active tab's panes reach the grid; the rest
//! keep running exactly as a minimized pane does — the PTY reads, the agent
//! works, the nav row still blinks — they just are not drawn.
//!
//! Pane numbers stay global (`crew ask 3`, Cmd+3 and the nav rows all mean
//! the same pane whichever tab it is in), and focusing a pane in another tab
//! brings its tab forward: the same "every focus path is a restore path" rule
//! the nav's minimized panes live by (`reconcile_grid`).
//!
//! Each tab keeps its own [`GridLayout`] while it is in the background, so
//! coming back lands on the tile you left, with its pins, rather than a grid
//! rebuilt in index order.
use crate::grid::GridLayout;

/// One tab: its id, its `/tab rename` name, and its grid while parked.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tab {
    pub id: u32,
    pub name: Option<String>,
    /// The tab's LRU while another tab is active; the live one is `app.grid`.
    pub grid: GridLayout,
}

/// Every tab, in nav order, and which one is on screen.
#[derive(Debug, Clone)]
pub(crate) struct Tabs {
    pub list: Vec<Tab>,
    pub active: u32,
    next: u32,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            list: vec![Tab {
                id: 1,
                ..Tab::default()
            }],
            active: 1,
            next: 2,
        }
    }
}

impl Tabs {
    /// Insert a fresh tab after the active one and return its id. It is not
    /// activated: that is the caller's move, since it swaps grids.
    pub(crate) fn add(&mut self) -> u32 {
        let at = self.pos(self.active).map_or(self.list.len(), |p| p + 1);
        self.insert(at)
    }

    /// A fresh tab at the end of the list (`/tab move` past the last).
    pub(crate) fn push(&mut self) -> u32 {
        self.insert(self.list.len())
    }

    fn insert(&mut self, at: usize) -> u32 {
        let id = self.next;
        self.next += 1;
        self.list.insert(
            at,
            Tab {
                id,
                ..Tab::default()
            },
        );
        id
    }

    /// Where tab `id` sits in the nav (0-based).
    pub(crate) fn pos(&self, id: u32) -> Option<usize> {
        self.list.iter().position(|t| t.id == id)
    }

    /// The id of the tab numbered `n` (1-based, as the nav shows it).
    pub(crate) fn id_at(&self, n: usize) -> Option<u32> {
        self.list.get(n.checked_sub(1)?).map(|t| t.id)
    }

    /// The tab `dir` steps from the active one, wrapping.
    pub(crate) fn step(&self, dir: i32) -> Option<u32> {
        let n = i32::try_from(self.list.len()).ok().filter(|&n| n > 1)?;
        let at = self.pos(self.active)? as i32;
        Some(self.list[(at + dir).rem_euclid(n) as usize].id)
    }

    pub(crate) fn get_mut(&mut self, id: u32) -> Option<&mut Tab> {
        self.list.iter_mut().find(|t| t.id == id)
    }

    pub(crate) fn name(&self, id: u32) -> Option<&str> {
        self.list.iter().find(|t| t.id == id)?.name.as_deref()
    }

    /// Pane `idx` closed: every parked grid shifts the way `Vec::remove` did.
    pub(crate) fn on_close(&mut self, idx: usize) {
        for t in &mut self.list {
            t.grid.on_close(idx);
        }
    }

    /// Drop every tab no pane lives in, except the active one when it is
    /// the last (an empty crew still has a tab to open panes into). Returns
    /// the tab to fall back to when the active tab was among them: its left
    /// neighbour, else whatever is first.
    pub(crate) fn prune(&mut self, used: &[u32]) -> Option<u32> {
        let at = self.pos(self.active).unwrap_or(0);
        let active = self.active;
        self.list
            .retain(|t| used.contains(&t.id) || (t.id == active && used.is_empty()));
        if self.list.is_empty() {
            *self = Self::default();
            return None;
        }
        if self.pos(active).is_some() {
            return None;
        }
        let i = at.saturating_sub(1).min(self.list.len() - 1);
        Some(self.list[i].id)
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
