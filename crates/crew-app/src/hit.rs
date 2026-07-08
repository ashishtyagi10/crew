//! Cursor hit-testing: which docked surface or pane sits under the pointer.
use crate::app::{CrewApp, GAP};
use crate::chrome;

impl CrewApp {
    /// Focus the surface under the cursor: the input bar, or a grid pane.
    /// Returns the pane index when a pane was focused (for double-click handling).
    pub(crate) fn focus_at_cursor(&mut self) -> Option<usize> {
        if self.cursor_in_input() {
            self.input.focused = true;
            return None;
        }
        if let Some(i) = self.pane_at_sidebar().or_else(|| self.pane_at_cursor()) {
            self.focused = i;
            self.input.focused = false;
            return Some(i);
        }
        None
    }

    /// Which pane a click on the sidebar's PANES list targets, if any.
    pub(crate) fn pane_at_sidebar(&self) -> Option<usize> {
        if !self.config.show_nav {
            return None;
        }
        let (_cw, ch, _sw, sh, scale) = self.frame_geometry()?;
        // Same rect the rows are drawn in — shifted below the UPDATE card
        // while a `/update` runs, so clicks keep tracking the rows.
        let sb = chrome::stats_card_rect(sh, self.nav_px(scale), GAP, ch, self.update.is_some());
        if !chrome::point_in(sb, self.cursor.0, self.cursor.1) {
            return None;
        }
        let rel_row = ((self.cursor.1 - sb.y) / ch).floor() as u16;
        let idx = sidebar_pane_index(rel_row, self.sidebar.panes_top(self.log.len()))?;
        (idx < self.panes.len()).then_some(idx)
    }

    /// Which full tile's `[-]` minimize button sits under the cursor, if any.
    /// Zoomed, the one expanded tile carries the button; in the grid only the
    /// full tiles do (strip thumbnails draw none, so they are never tested).
    pub(crate) fn min_btn_at_cursor(&self) -> Option<usize> {
        let (cw, ch, _sw, _sh, _scale) = self.frame_geometry()?;
        let (content, placed) = self.placed_grid()?;
        let tiles = if self.zoomed {
            crate::render::frame_hit_rects(true, self.focused, self.panes.len(), content, placed)
        } else {
            placed.full
        };
        tiles.into_iter().find_map(|(idx, r)| {
            let hit = crate::panecard::min_btn_rect(r, cw, ch)?;
            chrome::point_in(hit, self.cursor.0, self.cursor.1).then_some(idx)
        })
    }

    /// Whether the cursor is over the docked input bar.
    pub(crate) fn cursor_in_input(&self) -> bool {
        let Some((_cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return false;
        };
        let ih = chrome::input_h(ch);
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        let ib = chrome::inputbar_rect(content, sh, ch, GAP);
        chrome::point_in(ib, self.cursor.0, self.cursor.1)
    }

    /// Which grid pane (if any) sits under the cursor — only inside the content
    /// area, so clicks on the sidebar or input bar do not steal focus. Covers
    /// both full-size tiles and minimized strip thumbnails.
    pub(crate) fn pane_at_cursor(&self) -> Option<usize> {
        let (_cw, ch, sw, sh, scale) = self.frame_geometry()?;
        let ih = chrome::input_h(ch);
        let c = chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        if !chrome::point_in(c, self.cursor.0, self.cursor.1) {
            return None;
        }
        self.pane_hit_rects()
            .into_iter()
            .find(|&(_, r)| chrome::point_in(r, self.cursor.0, self.cursor.1))
            .map(|(idx, _)| idx)
    }
}

/// Map a cursor cell-row measured from the sidebar card's OUTER top edge to a
/// pane-list index. The card content is inset one cell (the border row), the
/// `PANES` header sits at content row `panes_top`, and pane `k` is on the row
/// below it — so pane rows start at outer row `panes_top + 2`. `None` for the
/// border, header, and everything above.
fn sidebar_pane_index(rel_row: u16, panes_top: u16) -> Option<usize> {
    Some(rel_row.checked_sub(panes_top + 2)? as usize)
}

#[cfg(test)]
mod tests {
    use super::sidebar_pane_index;

    #[test]
    fn sidebar_rows_map_to_pane_indices() {
        let top = 21; // content row of the PANES header
                      // Border row 0 … header (outer row 22) → no pane.
        assert_eq!(sidebar_pane_index(0, top), None);
        assert_eq!(sidebar_pane_index(top + 1, top), None, "header row");
        // First pane row sits directly under the header.
        assert_eq!(sidebar_pane_index(top + 2, top), Some(0));
        assert_eq!(sidebar_pane_index(top + 3, top), Some(1));
        assert_eq!(sidebar_pane_index(top + 4, top), Some(2));
    }
}
