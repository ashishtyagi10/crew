use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

use super::diff::DiffLine;
use super::state::DiffViewState;

pub fn render_diff_view(frame: &mut Frame, state: &DiffViewState, _theme: &Theme) {
    let area = frame.area();

    let left_name = state
        .left_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let right_name = state
        .right_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let title = format!(" Diff: {} ↔ {} ", left_name, right_name);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan).bg(Color::Indexed(233)))
        .style(Style::default().bg(Color::Indexed(233)).fg(Color::White));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    // Reserve 1 line for hint bar
    let content_height = (inner.height - 1) as usize;
    let half_width = (inner.width / 2) as usize;

    // Header: left filename | right filename
    let header = Line::from(vec![
        Span::styled(
            format!(" {:<width$}", left_name, width = half_width - 2),
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Indexed(235))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "│",
            Style::default()
                .fg(Color::Rgb(60, 60, 65))
                .bg(Color::Indexed(235)),
        ),
        Span::styled(
            format!(" {}", right_name),
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Indexed(235))
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header),
        Rect {
            y: inner.y,
            height: 1,
            ..inner
        },
    );

    // Diff lines
    let visible = content_height.saturating_sub(1);
    let mut lines: Vec<Line<'_>> = Vec::with_capacity(visible);

    for idx in state.scroll_offset..(state.scroll_offset + visible).min(state.diff_lines.len()) {
        lines.push(render_diff_row(&state.diff_lines[idx], half_width));
    }

    // Fill remaining lines
    while lines.len() < visible {
        lines.push(Line::from(Span::styled(
            " ".repeat(inner.width as usize),
            Style::default().bg(Color::Indexed(233)),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines),
        Rect {
            y: inner.y + 1,
            height: visible as u16,
            ..inner
        },
    );

    // Hint bar
    let total = state.diff_lines.len();
    let pos = if total > 0 {
        state.scroll_offset + 1
    } else {
        0
    };
    let hint = format!(
        " Line {}/{} | Up/Down/PgUp/PgDn=Scroll | Esc=Close",
        pos, total
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(233)),
        )),
        Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        },
    );
}

fn render_diff_row(line: &DiffLine, half_width: usize) -> Line<'static> {
    let sep = Span::styled(
        "│",
        Style::default()
            .fg(Color::Rgb(60, 60, 65))
            .bg(Color::Indexed(233)),
    );
    match line {
        DiffLine::Same(text) => {
            let left_text = super::row::truncate_pad(text, half_width - 1);
            let right_text = super::row::truncate_pad(text, half_width - 1);
            Line::from(vec![
                Span::styled(
                    format!(" {}", left_text),
                    Style::default().fg(Color::White).bg(Color::Indexed(233)),
                ),
                sep,
                Span::styled(
                    format!(" {}", right_text),
                    Style::default().fg(Color::White).bg(Color::Indexed(233)),
                ),
            ])
        }
        DiffLine::Removed(text) => {
            let left_text = super::row::truncate_pad(text, half_width - 1);
            let right_text = " ".repeat(half_width - 1);
            Line::from(vec![
                Span::styled(
                    format!("-{}", left_text),
                    Style::default().fg(Color::Red).bg(Color::Indexed(52)),
                ),
                sep,
                Span::styled(
                    format!(" {}", right_text),
                    Style::default().fg(Color::DarkGray).bg(Color::Indexed(233)),
                ),
            ])
        }
        DiffLine::Added(text) => {
            let left_text = " ".repeat(half_width - 1);
            let right_text = super::row::truncate_pad(text, half_width - 1);
            Line::from(vec![
                Span::styled(
                    format!(" {}", left_text),
                    Style::default().fg(Color::DarkGray).bg(Color::Indexed(233)),
                ),
                sep,
                Span::styled(
                    format!("+{}", right_text),
                    Style::default().fg(Color::Green).bg(Color::Indexed(22)),
                ),
            ])
        }
        DiffLine::Changed(left, right) => {
            let left_text = super::row::truncate_pad(left, half_width - 1);
            let right_text = super::row::truncate_pad(right, half_width - 1);
            Line::from(vec![
                Span::styled(
                    format!("~{}", left_text),
                    Style::default().fg(Color::Red).bg(Color::Indexed(52)),
                ),
                sep,
                Span::styled(
                    format!("~{}", right_text),
                    Style::default().fg(Color::Green).bg(Color::Indexed(22)),
                ),
            ])
        }
    }
}
