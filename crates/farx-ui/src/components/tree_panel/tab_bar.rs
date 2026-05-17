use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render a tab bar at the top of the panel area (only when multiple tabs exist).
/// Returns the height consumed by the tab bar (0 or 1).
pub fn render_tab_bar(
    frame: &mut Frame,
    area: Rect,
    tabs: &[(String, bool)],
    is_active: bool,
    theme: &Theme,
) -> u16 {
    if tabs.len() <= 1 {
        return 0;
    }

    let tab_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };

    let mut spans: Vec<Span<'_>> = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(Color::Indexed(235))));

    for (i, (name, active)) in tabs.iter().enumerate() {
        let truncated: String = name.chars().take(12).collect();
        let label = format!(" {} ", truncated);

        let style = if *active && is_active {
            Style::default()
                .fg(Color::White)
                .bg(theme.panel_bg)
                .add_modifier(Modifier::BOLD)
        } else if *active {
            Style::default().fg(Color::White).bg(Color::Indexed(238))
        } else {
            Style::default()
                .fg(Color::Rgb(140, 140, 150))
                .bg(Color::Indexed(235))
        };

        let idx_style = if *active && is_active {
            Style::default()
                .fg(Color::Yellow)
                .bg(theme.panel_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray).bg(if *active {
                Color::Indexed(238)
            } else {
                Color::Indexed(235)
            })
        };

        spans.push(Span::styled(format!("{}", i + 1), idx_style));
        spans.push(Span::styled(label, style));
        spans.push(Span::styled(
            "│",
            Style::default()
                .fg(Color::Rgb(60, 60, 65))
                .bg(Color::Indexed(235)),
        ));
    }

    // Fill remaining width
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let remaining = (area.width as usize).saturating_sub(used);
    if remaining > 0 {
        spans.push(Span::styled(
            " ".repeat(remaining),
            Style::default().bg(Color::Indexed(235)),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), tab_area);
    1
}
