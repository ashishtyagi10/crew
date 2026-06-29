//! Rolling history + sparkline rendering for the sidebar. A [`History`] keeps the
//! last N samples; [`sparkline_cells`] draws them as a one-row ratatui
//! `Sparkline` converted to Crew cells. The chart "moves" as samples are pushed
//! on the sidebar's existing ~1 Hz refresh — it costs nothing beyond the repaint
//! that already happens each second, so animation never compromises performance.
use std::collections::VecDeque;

use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Sparkline, Widget};

/// Fixed-capacity ring of recent samples (oldest at the front, newest at back).
pub struct History {
    cap: usize,
    data: VecDeque<u64>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            cap,
            data: VecDeque::with_capacity(cap),
        }
    }

    /// Append a sample, dropping the oldest once capacity is reached.
    pub fn push(&mut self, v: u64) {
        if self.data.len() == self.cap {
            self.data.pop_front();
        }
        self.data.push_back(v);
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The most recent `width` samples (or fewer), oldest first — what fills the
    /// chart left→right so the newest reading sits at the right edge.
    fn tail(&self, width: usize) -> Vec<u64> {
        let start = self.data.len().saturating_sub(width);
        self.data.iter().skip(start).copied().collect()
    }
}

/// Render `hist` as a single-row sparkline `width` cells wide, its left edge at
/// `col0` on `row`. `max` scales the bars (e.g. 100 for a percentage). Empty when
/// there's no history or no width.
pub fn sparkline_cells(
    hist: &History,
    width: u16,
    col0: u16,
    row: u16,
    max: u64,
    fg: (u8, u8, u8),
) -> Vec<CellView> {
    if width == 0 || hist.is_empty() {
        return Vec::new();
    }
    let data = hist.tail(width as usize);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, 1));
    Sparkline::default()
        .data(&data)
        .max(max.max(1))
        .style(Style::default().fg(Color::Rgb(fg.0, fg.1, fg.2)))
        .render(buf.area, &mut buf);
    let mut cells = crate::tui::to_cells(&buf);
    for c in &mut cells {
        c.col += col0;
        c.row += row;
    }
    cells
}

/// Sidebar convenience: render `hist` as a percentage (0–100) sparkline indented
/// under the section legend (col 3), spanning the rest of `cols` on `row`.
pub fn cpu_row(hist: &History, cols: u16, row: u16) -> Vec<CellView> {
    if cols <= 5 {
        return Vec::new();
    }
    let fg = crate::palette::accent();
    sparkline_cells(hist, cols.saturating_sub(4), 3, row, 100, fg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_caps_and_keeps_newest() {
        let mut h = History::new(3);
        for v in [1, 2, 3, 4, 5] {
            h.push(v);
        }
        // capacity 3 keeps the three newest, oldest first
        assert_eq!(h.tail(10), vec![3, 4, 5]);
    }

    #[test]
    fn tail_returns_at_most_width() {
        let mut h = History::new(10);
        for v in [10, 20, 30, 40] {
            h.push(v);
        }
        assert_eq!(h.tail(2), vec![30, 40]);
    }

    #[test]
    fn empty_history_renders_nothing() {
        let h = History::new(8);
        assert!(sparkline_cells(&h, 8, 0, 0, 100, (0, 255, 160)).is_empty());
    }

    #[test]
    fn renders_block_glyphs_in_bounds() {
        let mut h = History::new(16);
        for v in [10, 40, 70, 100, 0, 55] {
            h.push(v);
        }
        let cells = sparkline_cells(&h, 8, 3, 4, 100, (0, 255, 160));
        assert!(!cells.is_empty());
        // shifted to the requested origin and within the requested width
        assert!(cells.iter().all(|c| c.row == 4));
        assert!(cells.iter().all(|c| (3..3 + 8).contains(&c.col)));
        // a full-height sample yields the tallest block
        assert!(cells.iter().any(|c| c.c == '█'));
    }
}
