use farx_core::tree::TreeState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Returns the height consumed by the filter bar (0 or 1).
pub(super) fn filter_height(tree: &TreeState, filter_editing: bool) -> u16 {
    if !tree.filter.is_empty() || filter_editing {
        1
    } else {
        0
    }
}

/// Render the filter bar inside the panel. Caller must already know it's needed.
pub(super) fn render_filter_bar(
    frame: &mut Frame,
    inner: Rect,
    tree: &TreeState,
    filter_editing: bool,
) {
    let filter_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    let filter_display = format!(
        " Filter: {:<width$}",
        tree.filter,
        width = (inner.width as usize).saturating_sub(10)
    );
    let filter_style = if filter_editing {
        Style::default().fg(Color::Yellow).bg(Color::Indexed(238))
    } else {
        Style::default().fg(Color::Cyan).bg(Color::Indexed(237))
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(filter_display, filter_style))),
        filter_area,
    );
    if filter_editing {
        frame.set_cursor_position((inner.x + 9 + tree.filter.chars().count() as u16, inner.y));
    }
}
