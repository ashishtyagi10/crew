use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use super::state::ViewerState;

pub(super) fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    state: &ViewerState,
    content_height: usize,
) {
    let status_y = area.y + area.height.saturating_sub(1);
    let percentage = if state.total_lines == 0 {
        100
    } else {
        ((state.scroll_offset + content_height).min(state.total_lines) * 100) / state.total_lines
    };
    let follow_indicator = if state.follow { "FOLLOW  " } else { "" };
    let status_text = if let Some(ref input) = state.search_input {
        format!(" Search: {}_ (Enter=Find, Esc=Cancel) ", input)
    } else if let Some(ref input) = state.goto_input {
        format!(" Go to line: {}_ (Enter=Go, Esc=Cancel) ", input)
    } else {
        let ext = state.file_path.extension().and_then(|e| e.to_str());
        let is_md = matches!(ext, Some("md" | "markdown" | "mdx"));
        let md_hint = if is_md {
            if state.markdown_mode {
                "  Ctrl+M=Raw"
            } else {
                "  Ctrl+M=Preview"
            }
        } else {
            ""
        };
        format!(
            " Line {}/{} ({}%) | {}{}{}Esc/F3/q=Close  Ctrl+W=Wrap  Ctrl+G=GoTo{}",
            state.scroll_offset + 1,
            state.total_lines,
            percentage,
            if state.wrap { "Wrap " } else { "" },
            if state.markdown_mode { "MD " } else { "" },
            follow_indicator,
            md_hint,
        )
    };
    let status_line = Line::from(Span::styled(
        status_text,
        Style::default()
            .fg(Color::Rgb(16, 16, 18))
            .bg(Color::Rgb(220, 170, 60)),
    ));
    // Render over the bottom border
    frame.render_widget(
        Paragraph::new(status_line),
        Rect::new(area.x, status_y, area.width, 1),
    );
}
