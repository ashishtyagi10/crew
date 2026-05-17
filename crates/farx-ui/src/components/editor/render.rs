use super::{EditorMode, EditorState};
use crate::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_editor(frame: &mut Frame, state: &mut EditorState, _theme: &Theme) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let file_name = state
        .file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "new file".to_string());
    let modified_marker = if state.modified { " [modified]" } else { "" };
    let preview_marker = if state.preview_mode { " [preview]" } else { "" };
    let title = format!(" Edit: {}{}{} ", file_name, modified_marker, preview_marker);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Rgb(200, 200, 210))
                .bg(Color::Rgb(22, 22, 26)),
        )
        .style(
            Style::default()
                .bg(Color::Rgb(22, 22, 26))
                .fg(Color::Rgb(200, 200, 210)),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_height = inner.height.saturating_sub(1) as usize; // -1 for status bar
    let gutter_width = 6u16;
    let text_width = inner.width.saturating_sub(gutter_width) as usize;

    if state.preview_mode {
        super::render_preview::render(frame, state, area, inner, visible_height);
        return;
    }

    // Adjust scroll so cursor is visible
    state.scroll_to_cursor(visible_height, inner.width as usize);

    if state.wrap && text_width > 0 {
        super::render_wrap::render(
            frame,
            state,
            area,
            inner,
            visible_height,
            gutter_width,
            text_width,
        );
    } else {
        super::render_nowrap::render(
            frame,
            state,
            area,
            inner,
            visible_height,
            gutter_width,
            text_width,
        );
    }

    render_status_bar(frame, state, inner);
}

fn render_status_bar(frame: &mut Frame, state: &EditorState, inner: Rect) {
    let status_y = inner.y + inner.height.saturating_sub(1);
    let is_md = state.is_markdown_file();
    let status = match state.mode {
        EditorMode::ConfirmExit => {
            " File modified. Save? (Y)es / (N)o / (S)ave and exit / (Esc) cancel ".to_string()
        }
        EditorMode::Search => {
            format!(" Search: {}_  (Enter=Find, Esc=Cancel)", state.search_query)
        }
        EditorMode::GotoLine => {
            format!(
                " Go to line: {}_  (Enter=Go, Esc=Cancel)",
                state.goto_line_input
            )
        }
        EditorMode::Normal => {
            let md_hint = if is_md { "  Ctrl+M=Preview" } else { "" };
            format!(
                " Ln {}, Col {} | {} | {}Ctrl+S=Save  Ctrl+W=Wrap  Ctrl+G=GoTo{}",
                state.cursor_line + 1,
                state.cursor_col + 1,
                if state.modified { "Modified" } else { "Saved" },
                if state.wrap { "Wrap " } else { "" },
                md_hint,
            )
        }
    };
    let status_line = Line::from(Span::styled(
        format!("{:<width$}", status, width = inner.width as usize),
        Style::default()
            .fg(Color::Rgb(16, 16, 18))
            .bg(Color::Rgb(220, 170, 60)),
    ));
    frame.render_widget(
        Paragraph::new(status_line),
        Rect::new(inner.x, status_y, inner.width, 1),
    );

    if state.mode == EditorMode::Search {
        let cursor_x = inner.x + 9 + state.search_cursor as u16;
        frame.set_cursor_position((cursor_x.min(inner.x + inner.width - 1), status_y));
    } else if state.mode == EditorMode::GotoLine {
        let cursor_x = inner.x + 14 + state.goto_line_input.len() as u16;
        frame.set_cursor_position((cursor_x.min(inner.x + inner.width - 1), status_y));
    }
}
