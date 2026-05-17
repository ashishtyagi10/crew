use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::data::InfoPanelData;
use super::sections::{append_dir_section, append_disk_section, append_preview_section};
use crate::theme::Theme;

pub fn render_info_panel(frame: &mut Frame, area: Rect, data: &InfoPanelData, _theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Info ")
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

    let mut lines = Vec::new();
    let styles = SectionStyles::new();

    append_dir_section(&mut lines, data, &styles);
    append_disk_section(&mut lines, data, &styles);
    append_preview_section(&mut lines, data, &styles, inner);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Ctrl+L to close", styles.dim)));

    frame.render_widget(Paragraph::new(lines), inner);
}

pub(super) struct SectionStyles {
    pub label: Style,
    pub value: Style,
    pub dim: Style,
}

impl SectionStyles {
    fn new() -> Self {
        Self {
            label: Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(22, 22, 26)),
            value: Style::default().fg(Color::White).bg(Color::Rgb(22, 22, 26)),
            dim: Style::default()
                .fg(Color::Rgb(200, 200, 210))
                .bg(Color::Rgb(22, 22, 26)),
        }
    }
}
