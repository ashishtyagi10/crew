//! The Explorer section in the app: its root, and what a click, the wheel or the pointer means
//! over it. The rows are [`crate::explorerdraw`]'s — row 0 the rule, item `k` of the scrolled
//! list on row `k + 1` — read through the panel rect every nav hit path uses.
use std::path::PathBuf;

use crate::app::CrewApp;
use crate::navsection::NavSection;

impl CrewApp {
    /// The tree's root: the focused pane's directory, where the shell being typed in stands —
    /// crew's own when that pane has none.
    pub(crate) fn explorer_root(&self) -> PathBuf {
        let kept = &self.sidebar.explorer.root;
        let here = (self.panes.get(self.focused))
            .and_then(|p| p.dir.clone())
            .filter(|d| d.is_dir());
        if let Some(d) = &here {
            kept.replace(Some(d.clone()));
        }
        // A pane with no directory of its own — a viewer the tree just opened — keeps the
        // root the tree already had.
        here.or_else(|| kept.borrow().clone().filter(|d| d.is_dir()))
            .or_else(|| Some(self.cwd.clone()).filter(|d| d.is_dir()))
            // Before anything has set a directory: where the process stands.
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default()
    }

    /// The panel's content row under the cursor and its rows, when the open section is the
    /// Explorer.
    fn explorer_row(&self) -> Option<(u16, u16)> {
        self.section_row(NavSection::Explorer)
    }

    /// The tree item under the cursor, as an index into [`crate::explorer::Explorer::items`].
    pub(crate) fn explorer_item_at_cursor(&self) -> Option<usize> {
        let (row, _) = self.explorer_row()?;
        let len = self.explorer_items().len();
        let k = usize::from(row.checked_sub(crate::explorerdraw::TOP)?) + scrolled(self, len);
        (k < len).then_some(k)
    }

    /// A press on the tree: a folder opens or closes, a file opens in the viewer, the "n more"
    /// line opens its folder in Far. `true` when it was the tree's.
    pub(crate) fn explorer_press(&mut self) -> bool {
        // A pinned root's `×` first: the narrower target, or the root's toggle would claim it.
        if let Some(k) = self.explorer_x_at_cursor() {
            return self.explorer_unpin_at(k);
        }
        let Some(k) = self.explorer_item_at_cursor() else {
            return false;
        };
        let items = self.explorer_items();
        let Some(item) = items.get(k) else {
            return false;
        };
        match (item.dir, item.more) {
            // The "n more" line: the tree keeps CAP entries a folder, Far lists them all — so
            // the rest of this folder opens there, where they can be scrolled and sorted.
            // Browsing goes on in Far, and the tree's hold on the keys ends with the click: Far
            // may be the very pane that held them, so the focus change alone would not end it.
            (_, Some(_)) => {
                self.far_open_at(&item.path);
                self.sidebar.explorer.keys = None;
            }
            (true, None) => self.sidebar.explorer.toggle(item),
            (false, None) => {
                let path = item.path.clone();
                self.sidebar.explorer.picked = Some(path.clone());
                self.explorer_open(path);
            }
        }
        self.redraw();
        true
    }

    /// Open `path` in the tree's preview viewer: the one it opened last, given this file
    /// instead — unless that pane is gone or holds unsaved edits, when a new viewer opens beside
    /// the panes on screen (not in a tab of its own: a tab per click buries the grid).
    pub(crate) fn explorer_open(&mut self, path: PathBuf) {
        let preview = self.sidebar.explorer.preview;
        let reuse = self.panes.iter().position(|p| {
            Some(p.born_ms) == preview
                && matches!(&p.content, crate::pane::PaneContent::View(v) if !v.dirty)
        });
        if let Some(i) = reuse {
            if let crate::pane::PaneContent::View(v) = &mut self.panes[i].content {
                *v = crate::viewrecent::open(path);
            }
            self.focused = i;
            self.input.focused = false;
            self.reconcile_grid();
            return;
        }
        let before = self.panes.len();
        self.open_view_here(&path.to_string_lossy());
        if self.panes.len() > before {
            self.sidebar.explorer.preview = self.panes.last().map(|p| p.born_ms);
        }
    }

    /// A wheel over the tree scrolls it. `true` when the wheel was the tree's.
    pub(crate) fn explorer_scroll(&mut self, lines: i32) -> bool {
        let Some((_, rows)) = self.explorer_row() else {
            return false;
        };
        let len = self.explorer_items().len();
        let shown = usize::from(rows.saturating_sub(crate::explorerdraw::TOP));
        self.sidebar.explorer.scroll_by(lines, len, shown);
        true
    }

    /// The tree's cells for a panel `cols` × `rows`.
    pub(crate) fn explorer_cells(&self, cols: u16, rows: u16) -> Vec<crew_render::CellView> {
        let e = &self.sidebar.explorer;
        let items = self.explorer_items();
        let hover = self.explorer_item_at_cursor();
        let gits: Vec<_> = (self.explorer_roots().iter())
            .filter_map(|r| self.git_fleet.files(r))
            .collect();
        let x = hover
            .filter(|&k| Self::explorer_pinned_at(&items, k).is_some())
            .map(|k| (k, self.nav_x_under_cursor()));
        let scroll = scrolled(self, items.len());
        let cursor = self
            .tree_has_keys()
            .then(|| e.keys.map(|k| k.row))
            .flatten();
        let marks = crate::explorerdraw::Marks {
            hover,
            picked: e.picked.as_deref(),
            cursor,
            git: &gits,
            x,
            find: cursor.and(e.find.live(crate::explorerfind::now())),
        };
        crate::explorerdraw::cells(&items, scroll, marks, (cols, rows))
    }
}

/// The scroll the tree is drawn at — and so the one a click reads back: a root with fewer rows
/// than the last one was scrolled for shows from as far down as it goes.
fn scrolled(app: &CrewApp, len: usize) -> usize {
    app.sidebar.explorer.scroll.min(len.saturating_sub(1))
}

/// The tree row last drawn under the pointer (`usize::MAX`: none) — so a move from one row to
/// the next repaints, as the nav's other rows do (`panehover`).
static HOVER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

/// Publish this move's tree hover. `true` when it changed.
pub(crate) fn publish_hover(k: Option<usize>) -> bool {
    let v = k.unwrap_or(usize::MAX);
    HOVER.swap(v, std::sync::atomic::Ordering::Relaxed) != v
}

#[cfg(test)]
#[path = "explorerhit_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "explorermore_tests.rs"]
mod more_tests;
