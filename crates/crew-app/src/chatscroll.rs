//! Scroll affordances for the crew pane's message area: a proportional
//! scrollbar in the last column while the transcript overflows, and a
//! right-aligned `↓ N new` pill when messages arrive while scrolled up —
//! scrolling back to the bottom clears it.
use crew_render::CellView;

impl crate::chat::ChatPane {
    /// Scroll the message history by `delta` lines (positive = up/older),
    /// clamped to the available scrollback for the current width/height.
    pub fn scroll(&mut self, delta: i32, cols: u16, rows: u16) {
        // The header/roster rows and the composer sit outside the message
        // area, and a live swarm block (if any) claims rows from the bottom
        // of the message area too. `msg_rows_budget` is the single source
        // for all three, shared with the actual drawn window, so the clamp
        // can never drift from what's on screen (the tiny-pane plain
        // fallback never draws a block, so it keeps its own row math).
        let top = self.top_rows(rows);
        let msg_rows = if top == 0 {
            rows.saturating_sub(1) as usize
        } else {
            crate::chatplace::msg_rows_budget(self, cols, rows) as usize
        };
        // The card view (normal panes) and the plain fallback (tiny panes)
        // wrap to different line counts; clamp against whichever is shown.
        let total = if top == 0 {
            crate::chatlayout::wrapped_line_count(&self.messages, cols)
        } else {
            let view = crate::chatmsgs::View {
                source: self.show_source,
                compact: self.compact_view,
                gap_rows: crate::density::level().card_gap_rows(),
                streaming_from: self.messages.len(),
            };
            let visible = self.visible_messages();
            crate::chatmsgs::card_line_count(&visible, cols, view)
        };
        let max = total.saturating_sub(msg_rows);
        let next = self.scroll as i64 + delta as i64;
        self.scroll = next.clamp(0, max as i64) as usize;
        if self.scroll == 0 {
            self.unread = 0; // back at the live bottom — nothing is "new"
        }
    }
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bold: bool) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: crew_theme::theme().page_bg,
        bold,
        italic: false,
        ..Default::default()
    }
}

/// Proportional scroll-thumb geometry for a `visible`-row window into `total`
/// rows whose first visible row is `first` (0-based from the top): the
/// thumb's `(offset, length)` in window rows. `None` when everything fits —
/// shared by the chat scrollbar and the far panels' border thumbs.
pub(crate) fn thumb(total: usize, visible: usize, first: usize) -> Option<(usize, usize)> {
    if total <= visible || visible == 0 {
        return None;
    }
    let len = ((visible * visible).div_ceil(total)).max(1);
    Some((first * visible / total, len))
}

/// The `↓ N new` pill, right-aligned at `row`. Empty when nothing is unread.
pub(crate) fn new_pill_cells(unread: usize, cols: u16, row: u16) -> Vec<CellView> {
    if unread == 0 {
        return Vec::new();
    }
    let label = format!("\u{2193} {unread} new");
    let w = label.chars().count() as u16;
    if cols <= w {
        return Vec::new();
    }
    let accent = crate::palette::accent();
    (cols - w - 1..)
        .zip(label.chars())
        .map(|(x, c)| cell(x, row, c, accent, true))
        .collect()
}

#[cfg(test)]
#[path = "chatscroll_tests.rs"]
mod tests;
