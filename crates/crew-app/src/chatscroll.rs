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
            };
            crate::chatmsgs::card_line_count(&self.messages, cols, view)
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

/// A proportional scrollbar for a `visible`-row window into `total` lines,
/// `scroll` lines up from the bottom, drawn in column `col` over the message
/// rows `top..top+visible`. Empty when nothing overflows.
pub(crate) fn scrollbar_cells(
    total: usize,
    visible: usize,
    scroll: usize,
    col: u16,
    top: u16,
) -> Vec<CellView> {
    // First content line in the window, 0-based from the transcript top.
    let first = total.saturating_sub(visible) - scroll.min(total.saturating_sub(visible));
    let Some((thumb_top, thumb_len)) = thumb(total, visible, first) else {
        return Vec::new();
    };
    let t = crew_theme::theme();
    (0..visible)
        .map(|i| {
            let in_thumb = i >= thumb_top && i < thumb_top + thumb_len;
            if in_thumb {
                cell(col, top + i as u16, '\u{2503}', t.text_muted, true) // ┃
            } else {
                cell(col, top + i as u16, '\u{2502}', t.dim, false) // │
            }
        })
        .collect()
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
mod tests {
    use super::*;
    use crate::chat::ChatPane;
    use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};
    use crew_plugin::Plugin;

    #[test]
    fn no_scrollbar_when_content_fits() {
        assert!(scrollbar_cells(5, 10, 0, 79, 2).is_empty());
        assert!(scrollbar_cells(10, 10, 0, 79, 2).is_empty());
    }

    #[test]
    fn scroll_clamp_accounts_for_the_live_swarm_block() {
        // A long transcript on a live 8-task swarm run: msg_rows_budget
        // reserves rows for the block, so the drawn window is shorter than
        // `rows - top - bottom` alone suggests. If the scroll clamp doesn't
        // account for the block too, the top of the transcript becomes
        // unreachable — max scroll stops short of the first line.
        let plugin =
            Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
        let mut p = ChatPane::new(plugin, "crew".into());
        for i in 0..100 {
            p.messages.push(crate::chatlayout::Message {
                sender: "crew".into(),
                text: format!("message number {i}"),
                ts: String::new(),
                meta: String::new(),
            });
        }
        let tasks = (0..8)
            .map(|i| TaskSpec {
                id: TaskId(i),
                title: format!("task-{i}"),
                agent: AgentKind::Api { system: None },
                model: ModelTier::Cheap,
                deps: vec![],
                prompt: "p".into(),
                specialty: String::new(),
                expertise: String::new(),
            })
            .collect();
        p.absorb_hive_plan(tasks);

        let (cols, rows) = (80u16, 30u16);
        p.scroll(1_000_000, cols, rows);
        let lines = crate::chatplace::placed_lines(&p, cols, rows);
        let visible: String = lines
            .iter()
            .flat_map(|(_, l)| l.iter().map(|c| c.c))
            .collect();
        assert!(
            visible.contains("message number 0"),
            "max scroll should reach the very first transcript line even \
             while a live swarm block is open; visible window: {visible:?}"
        );
    }

    #[test]
    fn thumb_geometry_is_proportional_and_anchored() {
        assert_eq!(thumb(5, 10, 0), None, "fits — no thumb");
        assert_eq!(thumb(10, 10, 0), None, "exactly fits — no thumb");
        assert_eq!(thumb(100, 10, 0), Some((0, 1)), "top of the list");
        let (top, len) = thumb(100, 10, 90).expect("overflowing");
        assert_eq!(top + len, 10, "bottom-anchored at max scroll");
        let (top, len) = thumb(100, 10, 45).expect("overflowing");
        assert!(top > 0 && top + len < 10, "mid-scroll sits mid-track");
    }

    #[test]
    fn thumb_sits_at_bottom_when_following_live() {
        let cells = scrollbar_cells(100, 10, 0, 79, 2);
        assert_eq!(cells.len(), 10);
        let thumb: Vec<u16> = cells
            .iter()
            .filter(|c| c.c == '\u{2503}')
            .map(|c| c.row)
            .collect();
        assert!(!thumb.is_empty());
        assert_eq!(*thumb.last().unwrap(), 11, "thumb hugs the window bottom");
    }

    #[test]
    fn thumb_moves_to_top_when_fully_scrolled() {
        let cells = scrollbar_cells(100, 10, 90, 79, 2);
        let first_thumb = cells.iter().find(|c| c.c == '\u{2503}').unwrap();
        assert_eq!(first_thumb.row, 2, "thumb at the window top");
    }

    #[test]
    fn pill_is_right_aligned_and_gated_on_unread() {
        assert!(new_pill_cells(0, 80, 5).is_empty());
        let cells = new_pill_cells(3, 80, 5);
        let text: String = cells.iter().map(|c| c.c).collect();
        assert_eq!(text, "\u{2193} 3 new");
        assert_eq!(cells.last().unwrap().col, 78); // one column in from the edge
        assert!(cells.iter().all(|c| c.row == 5));
    }

    #[test]
    fn pill_hides_when_too_narrow() {
        assert!(new_pill_cells(3, 6, 0).is_empty());
    }
}
