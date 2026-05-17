use crate::theme::Theme;
use farx_core::tree::TreeState;
use farx_core::{SortField, SortOrder};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub(super) fn render_footer(frame: &mut Frame, footer_area: Rect, tree: &TreeState, theme: &Theme) {
    let node_count = tree.visible_nodes.len();
    let selected_count = tree.selected.len();
    let sort_label = match tree.sort_field {
        SortField::Name => "Name",
        SortField::Extension => "Ext",
        SortField::Size => "Size",
        SortField::Modified => "Date",
    };
    let sort_arrow = match tree.sort_order {
        SortOrder::Ascending => "↑",
        SortOrder::Descending => "↓",
    };
    let footer_text = if selected_count > 0 {
        format!(
            "  {} items | {} selected | {}{sort_arrow}",
            node_count, selected_count, sort_label
        )
    } else {
        format!("  {} items | {}{sort_arrow}", node_count, sort_label)
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(footer_text, theme.footer))),
        footer_area,
    );
}
