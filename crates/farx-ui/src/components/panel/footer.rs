//! Footer line: item counts, selection summary, quick-search indicator.

use farx_core::PanelState;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::helpers::format_size;
use crate::theme::Theme;

pub(super) fn render_footer(frame: &mut Frame, area: Rect, panel: &PanelState, theme: &Theme) {
    let file_count = panel.entries.len();
    let selected_count = panel.selected.len();
    let selected_size: u64 = panel
        .selected
        .iter()
        .filter_map(|&i| panel.entries.get(i))
        .map(|e| e.size)
        .sum();

    let footer_text = if selected_count > 0 {
        format!(
            " {file_count} items | {selected_count} selected ({}) ",
            format_size(selected_size)
        )
    } else {
        format!(" {file_count} items ")
    };

    let footer_text = if let Some(ref qs) = panel.quick_search {
        format!("{footer_text}  {qs}")
    } else {
        footer_text
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(footer_text, theme.footer))),
        area,
    );
}
