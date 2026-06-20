//! Rendering of the command line bordered box.

use std::path::Path;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use super::state::CommandLineState;
use crate::theme::Theme;

/// Render the command line as a bordered box.
///
/// Layout (3 rows total):
/// ┌─ /current/directory ──── Smart Command Line ─┐
/// │ > user input here                             │
/// └─ Type command or ask in English ── Esc=Clear ─┘
pub fn render_command_line(
    frame: &mut Frame,
    area: Rect,
    state: &CommandLineState,
    current_dir: &Path,
    status: &str,
    _theme: &Theme,
) {
    let has_input = !state.input.is_empty();

    // Detect mode for visual feedback
    let (mode_label, mode_color) = if !has_input {
        ("Ready", Color::Indexed(244))
    } else if looks_like_command(&state.input) {
        ("Shell", Color::Green)
    } else {
        ("AI", Color::Rgb(135, 215, 255))
    };

    let dir_str = current_dir.to_string_lossy();
    // Truncate dir if too long (use chars for safe unicode handling)
    let max_dir_len = (area.width as usize).saturating_sub(20);
    let dir_display = if dir_str.chars().count() > max_dir_len {
        let suffix: String = dir_str
            .chars()
            .skip(dir_str.chars().count().saturating_sub(max_dir_len))
            .collect();
        format!("...{}", suffix)
    } else {
        dir_str.to_string()
    };

    let border_color = if has_input {
        Color::Rgb(122, 162, 247)
    } else {
        Color::Indexed(238)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled(" ", Style::default().fg(border_color).bg(Color::Black)),
            Span::styled(
                &dir_display,
                Style::default().fg(Color::Yellow).bg(Color::Black),
            ),
            Span::styled(" ", Style::default().bg(Color::Black)),
        ]))
        .title_bottom(Line::from(vec![
            Span::styled(
                format!(" {} ", mode_label),
                Style::default()
                    .fg(Color::Black)
                    .bg(mode_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", status),
                Style::default().fg(Color::Indexed(244)).bg(Color::Black),
            ),
        ]))
        .border_style(Style::default().fg(border_color).bg(Color::Black))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    render_input_line(frame, inner, state, has_input);
}

fn render_input_line(frame: &mut Frame, inner: Rect, state: &CommandLineState, has_input: bool) {
    let prompt_style = Style::default()
        .fg(if has_input {
            Color::Rgb(122, 162, 247)
        } else {
            Color::Indexed(244)
        })
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let input_style = Style::default().fg(Color::White).bg(if has_input {
        Color::Indexed(235)
    } else {
        Color::Black
    });

    let prompt = "❯ ";
    let input_width = inner.width.saturating_sub(2) as usize; // 2 for "❯ "

    // Ghost text (suggestion) shown after the input in dim color
    let ghost = state.suggestion.as_deref().unwrap_or("");
    let ghost_style = Style::default().fg(Color::Indexed(240)).bg(if has_input {
        Color::Indexed(235)
    } else {
        Color::Black
    });

    let combined = format!("{}{}", state.input, ghost);
    let display = if combined.len() >= input_width {
        combined[..input_width].to_string()
    } else {
        format!("{:<width$}", combined, width = input_width)
    };

    // Split display into input part and ghost part
    let input_len = state.input.len().min(input_width);
    let mut spans = vec![
        Span::styled(prompt, prompt_style),
        Span::styled(display[..input_len].to_string(), input_style),
    ];
    if input_len < display.len() {
        spans.push(Span::styled(display[input_len..].to_string(), ghost_style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), inner);

    // Show cursor
    if has_input {
        let cursor_x = inner.x + 2 + state.cursor_pos as u16;
        if cursor_x < inner.x + inner.width {
            frame.set_cursor_position((cursor_x, inner.y));
        }
    }

    // Show Tab hint when suggestion is available
    if state.suggestion.is_some() && has_input {
        let hint_text = " Tab↹ ";
        let hint_x = inner.x + 2 + state.input.len() as u16 + ghost.len() as u16 + 1;
        if hint_x + hint_text.len() as u16 <= inner.x + inner.width {
            let hint_area = Rect::new(hint_x, inner.y, hint_text.len() as u16, 1);
            frame.render_widget(
                Paragraph::new(Span::styled(
                    hint_text,
                    Style::default()
                        .fg(Color::Rgb(220, 170, 60))
                        .bg(Color::Indexed(236)),
                )),
                hint_area,
            );
        }
    }
}

/// Quick heuristic: does the input look like a shell command?
/// (Mirrors the logic in app.rs smart_execute_command)
fn looks_like_command(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }
    let first_word = trimmed.split_whitespace().next().unwrap_or("");

    if first_word.starts_with('/') || first_word.starts_with("./") || first_word.starts_with("~/") {
        return true;
    }
    if trimmed.contains('|')
        || trimmed.contains('>')
        || trimmed.contains('<')
        || trimmed.contains("&&")
        || trimmed.contains("||")
    {
        return true;
    }

    const CMDS: &[&str] = &[
        "ls", "cd", "cp", "mv", "rm", "mkdir", "rmdir", "cat", "head", "tail", "grep", "find",
        "sed", "awk", "sort", "wc", "echo", "touch", "chmod", "pwd", "df", "du", "tar", "zip",
        "unzip", "curl", "wget", "ssh", "git", "docker", "make", "npm", "yarn", "cargo", "python",
        "python3", "pip", "node", "ruby", "go", "java", "brew", "apt", "sudo", "man", "vi", "vim",
        "nano", "code", "open", "clear", "which",
    ];
    CMDS.contains(&first_word)
}
