//! The nav's Far section: the folders Far panes went to last (`farrecent`), newest first, one
//! row each — the folder's name, then where it is, quieter. A click opens Far there: the open
//! Far pane's active panel goes to it, or a Far pane opens on it. Anywhere else below the rule
//! opens Far. More folders than the panel holds: the wheel scrolls them (`navlistscroll`).
//! A row's `×`, a gone folder and the open Far pane's folder are drawn in `navfarrow`.
use std::path::{Path, PathBuf};

use crew_render::CellView;

use crate::app::CrewApp;
use crate::navsection::NavSection;
use crate::pane::PaneContent;

/// Rows above the first folder: the rule.
pub(crate) const TOP: u16 = 1;

/// The folder on content row `row`, of `n` shown.
pub(crate) fn item_at(row: u16, n: usize) -> Option<usize> {
    let k = usize::from(row.checked_sub(TOP)?);
    (k < n).then_some(k)
}

/// The section's cells for a panel `cols` × `rows`: `(first, shown)` is the window of `list`
/// it shows, `marks` the hover, the `×`, the open Far pane's folder and the gone ones
/// (`navfarrow`).
pub(crate) fn cells(
    list: &[PathBuf],
    (first, shown): (usize, usize),
    marks: &crate::navfarrow::Marks,
    (cols, _rows): (u16, u16),
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let key = if list.is_empty() { "" } else { "recent" };
    let mut out = crate::boxdraw::section_header_key(
        "FAR",
        key,
        cols,
        t.border_normal,
        accent,
        t.text_muted,
        t.page_bg,
    );
    let max = cols.saturating_sub(1);
    let ind = crate::navtext::INDENT;
    if list.is_empty() {
        crate::navtext::put_at(&mut out, "no folders yet", ind, 1, max, t.text_muted);
        crate::navtext::put_at(&mut out, "click to open far", ind, 2, max, accent);
        return out;
    }
    for (k, dir) in list.iter().skip(first).take(shown).enumerate() {
        let look = marks.look(first + k, dir);
        crate::navfarrow::draw(&mut out, dir, TOP + k as u16, cols, look);
    }
    out
}

/// A folder's own name and where it stands (`~/code`); the root is its own name.
pub(crate) fn name_and_parent(dir: &Path) -> (String, String) {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.to_string_lossy().into_owned());
    let at = dir
        .parent()
        .map(crate::disktile::short_path)
        .unwrap_or_default();
    (name, at)
}

impl CrewApp {
    /// The recent folder under the pointer, as an index into `farrecent::list`.
    pub(crate) fn far_item_at_cursor(&self) -> Option<usize> {
        let (row, rows) = self.section_row(NavSection::Far)?;
        let (first, n) = self.list_window(NavSection::Far, rows);
        item_at(row, n).map(|k| first + k)
    }

    /// A press on the Far panel. `true` when it was the panel's.
    pub(crate) fn far_press(&mut self) -> bool {
        match self.section_row(NavSection::Far) {
            Some((row, _)) if row > 0 => {}
            _ => return false,
        }
        self.far_open_item(self.far_item_at_cursor());
        self.redraw();
        true
    }

    /// Open recent folder `k` (an index into `farrecent::list`) as a click on it does; `None`:
    /// Far, wherever it is. The click and the keyboard's Enter both come here.
    pub(crate) fn far_open_item(&mut self, k: Option<usize>) {
        match k.and_then(|k| crate::farrecent::list().get(k).cloned()) {
            Some(d) if !d.is_dir() => self.set_status(format!("far: {} is gone", d.display())),
            Some(d) => self.far_open_at(&d),
            None => self.far_open_at_none(),
        }
    }

    /// The Far pane a click goes to: the focused one, else the first; `None`: no Far pane.
    pub(crate) fn far_pane(&self) -> Option<usize> {
        let far = |i: &usize| matches!(self.panes[*i].content, PaneContent::Far(_));
        Some(self.focused)
            .filter(|i| *i < self.panes.len() && far(i))
            .or_else(|| (0..self.panes.len()).find(far))
    }

    /// Far at `dir`: the open Far pane's active panel goes there, or a Far pane opens.
    pub(crate) fn far_open_at(&mut self, dir: &Path) {
        let i = match self.far_pane() {
            Some(i) => i,
            None => {
                self.spawn_far_pane();
                self.panes.len() - 1
            }
        };
        if let PaneContent::Far(f) = &mut self.panes[i].content {
            f.go(dir);
        }
        self.focus_pane_at(i);
    }

    /// Far, wherever it is: the open Far pane focused, or a new one.
    fn far_open_at_none(&mut self) {
        match self.far_pane() {
            Some(i) => self.focus_pane_at(i),
            None => self.spawn_far_pane(),
        }
    }

    fn focus_pane_at(&mut self, i: usize) {
        self.focused = i;
        self.input.focused = false;
        self.reconcile_grid();
    }

    /// The Far section's cells for a panel `cols` × `rows`.
    pub(crate) fn far_cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        let win = self.list_window(NavSection::Far, rows);
        let marks = self.far_marks();
        let mut out = cells(&crate::farrecent::list(), win, &marks, (cols, rows));
        self.list_bar(&mut out, NavSection::Far, win, cols);
        out
    }
}

#[cfg(test)]
#[path = "navfar_tests.rs"]
pub(crate) mod tests;
