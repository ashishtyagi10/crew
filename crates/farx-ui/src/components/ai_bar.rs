use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

#[derive(Debug, Clone, PartialEq)]
pub enum AiBarAction {
    None,
    Close,
    Submit(String),
}

pub struct AiBarState {
    pub active: bool,
    pub input: String,
    pub cursor_pos: usize,
    pub response: Vec<String>,
    pub thinking: bool,
    pub scroll_offset: usize,
    pub copied: bool,
}

impl Default for AiBarState {
    fn default() -> Self {
        Self {
            active: true,
            input: String::new(),
            cursor_pos: 0,
            response: Vec::new(),
            thinking: false,
            scroll_offset: 0,
            copied: false,
        }
    }
}

impl AiBarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> AiBarAction {
        if self.thinking {
            // While thinking, only allow Esc to cancel
            if key.code == KeyCode::Esc {
                self.active = false;
                return AiBarAction::Close;
            }
            return AiBarAction::None;
        }

        // Ctrl+C: copy response to clipboard
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !self.response.is_empty() {
                let text = self.response.join("\n");
                copy_to_clipboard(&text);
                self.copied = true;
            }
            return AiBarAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.active = false;
                AiBarAction::Close
            }
            KeyCode::Enter => {
                if self.input.is_empty() {
                    AiBarAction::None
                } else {
                    let query = self.input.clone();
                    self.thinking = true;
                    self.copied = false;
                    AiBarAction::Submit(query)
                }
            }
            KeyCode::Char(ch) => {
                self.input.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
                AiBarAction::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
                AiBarAction::None
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.input.len() {
                    self.input.remove(self.cursor_pos);
                }
                AiBarAction::None
            }
            KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
                AiBarAction::None
            }
            KeyCode::Right => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.input.len());
                AiBarAction::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                AiBarAction::None
            }
            KeyCode::End => {
                self.cursor_pos = self.input.len();
                AiBarAction::None
            }
            KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                AiBarAction::None
            }
            KeyCode::Down => {
                self.scroll_offset += 1;
                AiBarAction::None
            }
            _ => AiBarAction::None,
        }
    }

    pub fn set_response(&mut self, text: String) {
        self.response = text.lines().map(String::from).collect();
        self.thinking = false;
        self.scroll_offset = 0;
    }

    pub fn append_response(&mut self, text: &str) {
        if self.response.is_empty() {
            self.response.push(String::new());
        }
        // Append text, handling newlines
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.response.push(String::new());
            }
            if let Some(last) = self.response.last_mut() {
                last.push_str(part);
            }
        }
    }
}

#[allow(unused_variables)]
pub fn render_ai_bar(frame: &mut Frame, state: &AiBarState, theme: &Theme) {
    let area = frame.area();

    // AI bar takes the bottom half of the screen
    let bar_height = (area.height / 2).max(8);
    let bar_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(bar_height),
        area.width,
        bar_height,
    );

    frame.render_widget(Clear, bar_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" AI Assistant (Ctrl+Space) ")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Rgb(135, 215, 255))
                .bg(Color::Indexed(234)),
        )
        .style(
            Style::default()
                .bg(Color::Indexed(234))
                .fg(Color::Rgb(135, 215, 255)),
        );

    let inner = block.inner(bar_area);
    frame.render_widget(block, bar_area);

    // Input line at top
    let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let prompt_style = Style::default()
        .fg(Color::Rgb(255, 175, 0))
        .bg(Color::Indexed(234));
    let input_style = Style::default().fg(Color::White).bg(Color::Indexed(236));

    let input_display = format!(
        "{:<width$}",
        state.input,
        width = (inner.width as usize).saturating_sub(4)
    );
    let input_line = Line::from(vec![
        Span::styled(" > ", prompt_style),
        Span::styled(input_display, input_style),
    ]);
    frame.render_widget(Paragraph::new(input_line), input_area);

    // Set cursor position in input
    frame.set_cursor_position((inner.x + 3 + state.cursor_pos as u16, inner.y));

    // Separator
    let sep_area = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let sep_line = Line::from(Span::styled(
        "\u{2500}".repeat(inner.width as usize),
        Style::default()
            .fg(Color::Indexed(240))
            .bg(Color::Indexed(234)),
    ));
    frame.render_widget(Paragraph::new(sep_line), sep_area);

    // Response area
    let response_area = Rect::new(
        inner.x,
        inner.y + 2,
        inner.width,
        inner.height.saturating_sub(3),
    );

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
        // Show placeholder
        let placeholder = vec![
            Line::from(Span::styled(
                " Ask me anything about your files...",
                Style::default()
                    .fg(Color::Indexed(244))
                    .bg(Color::Indexed(234)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Examples:",
                Style::default()
                    .fg(Color::Indexed(248))
                    .bg(Color::Indexed(234)),
            )),
            Line::from(Span::styled(
                "   \"Find all files larger than 100MB\"",
                Style::default()
                    .fg(Color::Indexed(244))
                    .bg(Color::Indexed(234)),
            )),
            Line::from(Span::styled(
                "   \"Organize photos by date\"",
                Style::default()
                    .fg(Color::Indexed(244))
                    .bg(Color::Indexed(234)),
            )),
            Line::from(Span::styled(
                "   \"Show me recently modified files\"",
                Style::default()
                    .fg(Color::Indexed(244))
                    .bg(Color::Indexed(234)),
            )),
            Line::from(Span::styled(
                "   \"Rename all .jpeg files to .jpg\"",
                Style::default()
                    .fg(Color::Indexed(244))
                    .bg(Color::Indexed(234)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Powered by OpenRouter (free models) - set OPENROUTER_API_KEY",
                Style::default()
                    .fg(Color::Indexed(240))
                    .bg(Color::Indexed(234)),
            )),
        ];
        frame.render_widget(Paragraph::new(placeholder), response_area);
    }

    // Bottom hint bar
    let hint_area = Rect::new(
        inner.x,
        inner.y + inner.height.saturating_sub(1),
        inner.width,
        1,
    );
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

/// Copy text to the system clipboard using platform-native commands.
fn copy_to_clipboard(text: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // macOS
    if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return;
    }

    // Linux (X11)
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return;
    }

    // Linux (Wayland)
    if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}
