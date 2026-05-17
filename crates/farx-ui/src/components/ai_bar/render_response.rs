use super::state::AiBarState;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub(super) fn render_response_area(frame: &mut Frame, state: &AiBarState, response_area: Rect) {
    if state.thinking {
        let dots = match (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            / 500)
            % 4
        {
            0 => ".",
            1 => "..",
            2 => "...",
            _ => "",
        };
        let thinking_line = Line::from(Span::styled(
            format!(" Thinking{}", dots),
            Style::default()
                .fg(Color::Rgb(255, 175, 0))
                .bg(Color::Indexed(234)),
        ));
        frame.render_widget(Paragraph::new(thinking_line), response_area);
    } else if !state.response.is_empty() {
        // Render response as markdown
        let full_text = state.response.join("\n");
        let md_lines = crate::components::markdown::render_markdown(&full_text);
        let visible_lines: Vec<Line> = md_lines
            .into_iter()
            .skip(state.scroll_offset)
            .take(response_area.height as usize)
            .collect();
        frame.render_widget(Paragraph::new(visible_lines), response_area);
    } else {
        frame.render_widget(Paragraph::new(placeholder_lines()), response_area);
    }
}

fn placeholder_lines() -> Vec<Line<'static>> {
    let dim = Style::default()
        .fg(Color::Indexed(244))
        .bg(Color::Indexed(234));
    let label = Style::default()
        .fg(Color::Indexed(248))
        .bg(Color::Indexed(234));
    let footer = Style::default()
        .fg(Color::Indexed(240))
        .bg(Color::Indexed(234));

    vec![
        Line::from(Span::styled(" Ask me anything about your files...", dim)),
        Line::from(""),
        Line::from(Span::styled(" Examples:", label)),
        Line::from(Span::styled("   \"Find all files larger than 100MB\"", dim)),
        Line::from(Span::styled("   \"Organize photos by date\"", dim)),
        Line::from(Span::styled("   \"Show me recently modified files\"", dim)),
        Line::from(Span::styled("   \"Rename all .jpeg files to .jpg\"", dim)),
        Line::from(""),
        Line::from(Span::styled(
            " Powered by OpenRouter (free models) - set OPENROUTER_API_KEY",
            footer,
        )),
    ]
}

pub(super) fn render_hint_bar(frame: &mut Frame, state: &AiBarState, hint_area: Rect) {
    let key_style = Style::default()
        .fg(Color::Rgb(255, 175, 0))
        .bg(Color::Indexed(234));
    let label_style = Style::default()
        .fg(Color::Indexed(244))
        .bg(Color::Indexed(234));

    let mut hint_spans = vec![
        Span::styled(" Enter", key_style),
        Span::styled("=Send  ", label_style),
        Span::styled("Esc", key_style),
        Span::styled("=Close  ", label_style),
        Span::styled("Up/Down", key_style),
        Span::styled("=Scroll  ", label_style),
        Span::styled("Ctrl+C", key_style),
        Span::styled("=Copy", label_style),
    ];

    if state.copied {
        hint_spans.push(Span::styled(
            "  Copied!",
            Style::default()
                .fg(Color::Green)
                .bg(Color::Indexed(234))
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(hint_spans)), hint_area);
}
