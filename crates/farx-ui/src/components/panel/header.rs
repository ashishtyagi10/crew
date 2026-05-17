//! Column header row + separator beneath it.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::helpers::pad_right;
use crate::theme::Theme;

/// Column widths used by the panel grid.
pub(super) struct ColumnWidths {
    pub name: usize,
    pub size: usize,
    pub date: usize,
}

impl ColumnWidths {
    pub fn compute(total_width: usize) -> Self {
        let sep_w: usize = 1;
        let size_col_w: usize = 8;
        let date_col_w: usize = 16;
        let name_col_w = total_width.saturating_sub(size_col_w + date_col_w + sep_w * 2);
        Self {
            name: name_col_w,
            size: size_col_w,
            date: date_col_w,
        }
    }
}

pub(super) fn render_header(frame: &mut Frame, inner: Rect, cols: &ColumnWidths, theme: &Theme) {
    let header_line = Line::from(vec![
        Span::styled(pad_right(" Name", cols.name), theme.column_header),
        Span::styled(theme.grid_separator, theme.grid_style),
        Span::styled(
            super::helpers::pad_left("Size", cols.size),
            theme.column_header,
        ),
        Span::styled(theme.grid_separator, theme.grid_style),
        Span::styled(pad_right("Modified", cols.date), theme.column_header),
    ]);

    let header_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(Paragraph::new(header_line), header_area);

    let sep_str = format!(
        "{}┼{}┼{}",
        "─".repeat(cols.name),
        "─".repeat(cols.size),
        "─".repeat(cols.date),
    );
    let sep_line = Line::from(Span::styled(sep_str, theme.grid_style));
    let sep_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(Paragraph::new(sep_line), sep_area);
}
