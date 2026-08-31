//! Where the todo list is scrolled to: paging, keeping the cursor in view,
//! and clamping to the ends.
//!
//! Split out of [`super`] for the line cap.

use super::TodoPane;

impl TodoPane {
    /// Mouse wheel: positive `lines` means up/older (see `scroll::scroll_pane`).
    /// Item-granular — one wheel line steps one item.
    pub(crate) fn scroll_wheel(&mut self, lines: i32, cols: u16, rows: u16) {
        self.scroll = self
            .scroll
            .saturating_add_signed(lines.saturating_neg() as isize);
        self.clamp_scroll(cols, rows);
    }

    /// Item indices in display order under the active filter — the done
    /// history's own ordering when the view is on.
    pub(crate) fn order(&self) -> Vec<usize> {
        if self.done_view {
            super::item::done_order(&self.items, self.filter.as_deref())
        } else {
            super::item::display_order(&self.items, self.filter.as_deref(), self.show_done)
        }
    }

    pub(crate) fn visible_len(&self) -> usize {
        self.order().len()
    }

    /// The selection one visible page forward or back from `sel`: successive
    /// item heights are summed against the list height, moving while they
    /// still fit the window — minimum one item, so an over-tall wrapped item
    /// never pins the selection. Filter-aware through [`Self::order`].
    pub(crate) fn page_target(&self, sel: usize, forward: bool, cols: u16, rows: u16) -> usize {
        let order = self.order();
        let n = order.len();
        if n == 0 {
            return 0;
        }
        let h = (super::render::list_height(self, cols, rows) as usize).max(1);
        let now_ms = crate::chattime::unix_now_ms();
        let mut acc = 0usize;
        let mut at = sel.min(n - 1);
        loop {
            let next = match (forward, at) {
                (true, a) if a + 1 < n => a + 1,
                (false, a) if a > 0 => a - 1,
                _ => break,
            };
            acc += super::render::row_h(&self.items, self.done_view, &order, next, cols, now_ms)
                as usize;
            if acc > h && at != sel.min(n - 1) {
                break;
            }
            at = next;
            if acc >= h {
                break;
            }
        }
        at
    }

    /// Keep the selection inside the list window after it moves. Items are
    /// variable-height (wrapped titles — [`super::render::item_h`]), so visibility
    /// is a row sum, not an item count.
    pub(crate) fn ensure_visible(&mut self, cols: u16, rows: u16) {
        let h = super::render::list_height(self, cols, rows) as usize;
        if h == 0 {
            return;
        }
        let order = self.order();
        let now_ms = crate::chattime::unix_now_ms();
        let dv = self.done_view;
        if let Some(s) = self.sel.filter(|&s| s < order.len()) {
            if s < self.scroll {
                self.scroll = s;
            } else {
                // Push the window down until items scroll..=s fit it (a
                // single over-tall item stops at its first row). Heights
                // are per display row — a day header rides on its item.
                let span = |from: usize| -> usize {
                    (from..=s)
                        .map(|di| {
                            super::render::row_h(&self.items, dv, &order, di, cols, now_ms) as usize
                        })
                        .sum()
                };
                while self.scroll < s && span(self.scroll) > h {
                    self.scroll += 1;
                }
            }
        }
        self.clamp_scroll(cols, rows);
    }

    /// Cap `scroll` at the smallest value that still fills the window with
    /// the list's tail.
    pub(crate) fn clamp_scroll(&mut self, cols: u16, rows: u16) {
        let h = super::render::list_height(self, cols, rows) as usize;
        let order = self.order();
        let now_ms = crate::chattime::unix_now_ms();
        let mut used = 0;
        let mut s = order.len();
        while s > 0 {
            let ih = super::render::row_h(&self.items, self.done_view, &order, s - 1, cols, now_ms)
                as usize;
            if used + ih > h {
                break;
            }
            used += ih;
            s -= 1;
        }
        self.scroll = self.scroll.min(s.min(order.len().saturating_sub(1)));
    }
}
