use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::Theme;

pub struct HelpState {
    pub active: bool,
    pub scroll_offset: usize,
}

impl Default for HelpState {
    fn default() -> Self {
        Self {
            active: true,
            scroll_offset: 0,
        }
    }
}

impl HelpState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('q') => {
                self.active = false;
                true // consumed
            }
            KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                true
            }
            KeyCode::Down => {
                self.scroll_offset += 1;
                true
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
                true
            }
            KeyCode::PageDown => {
                self.scroll_offset += 20;
                true
            }
            _ => true,
        }
    }
}

pub fn render_help(frame: &mut Frame, state: &HelpState, _theme: &Theme) {
    let area = frame.area();

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Farx Help - FAR Manager Keybindings ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Rgb(22, 22, 26)))
        .style(Style::default().bg(Color::Rgb(22, 22, 26)).fg(Color::Rgb(200, 200, 210)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_text = vec![
        Line::from(Span::styled(
            "  NAVIGATION",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab          ", Style::default().fg(Color::White)),
            Span::raw("Switch between panels"),
        ]),
        Line::from(vec![
            Span::styled("  Enter        ", Style::default().fg(Color::White)),
            Span::raw("Enter directory / open file"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+PgUp    ", Style::default().fg(Color::White)),
            Span::raw("Go to parent directory"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+PgDn    ", Style::default().fg(Color::White)),
            Span::raw("Enter directory"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+\\       ", Style::default().fg(Color::White)),
            Span::raw("Go to root directory"),
        ]),
        Line::from(vec![
            Span::styled("  Alt+F1/F2    ", Style::default().fg(Color::White)),
            Span::raw("Change drive/root (left/right panel)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  FILE OPERATIONS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  F3           ", Style::default().fg(Color::White)),
            Span::raw("View file"),
        ]),
        Line::from(vec![
            Span::styled("  F4           ", Style::default().fg(Color::White)),
            Span::raw("Edit file"),
        ]),
        Line::from(vec![
            Span::styled("  F5           ", Style::default().fg(Color::White)),
            Span::raw("Copy file(s) to other panel"),
        ]),
        Line::from(vec![
            Span::styled("  F6           ", Style::default().fg(Color::White)),
            Span::raw("Move/rename file(s)"),
        ]),
        Line::from(vec![
            Span::styled("  F7           ", Style::default().fg(Color::White)),
            Span::raw("Create directory"),
        ]),
        Line::from(vec![
            Span::styled("  F8           ", Style::default().fg(Color::White)),
            Span::raw("Delete file(s)"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+F4     ", Style::default().fg(Color::White)),
            Span::raw("Create new file"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+F5     ", Style::default().fg(Color::White)),
            Span::raw("Copy to same directory"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+F6     ", Style::default().fg(Color::White)),
            Span::raw("Rename file"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  SELECTION",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Insert       ", Style::default().fg(Color::White)),
            Span::raw("Select/deselect file"),
        ]),
        Line::from(vec![
            Span::styled("  Gray +       ", Style::default().fg(Color::White)),
            Span::raw("Select by mask"),
        ]),
        Line::from(vec![
            Span::styled("  Gray -       ", Style::default().fg(Color::White)),
            Span::raw("Deselect by mask"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  PANELS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+O       ", Style::default().fg(Color::White)),
            Span::raw("Toggle panels (show console)"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+L       ", Style::default().fg(Color::White)),
            Span::raw("Info panel"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  AI ASSISTANT",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+Space   ", Style::default().fg(Color::White)),
            Span::raw("Open AI command bar"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  OTHER",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  F1           ", Style::default().fg(Color::White)),
            Span::raw("Help (this screen)"),
        ]),
        Line::from(vec![
            Span::styled("  F9           ", Style::default().fg(Color::White)),
            Span::raw("Menu"),
        ]),
        Line::from(vec![
            Span::styled("  F10          ", Style::default().fg(Color::White)),
            Span::raw("Quit"),
        ]),
        Line::from(vec![
            Span::styled("  F11          ", Style::default().fg(Color::White)),
            Span::raw("Plugin commands"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Esc or F1 to close help",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    // Apply scroll
    let visible_lines: Vec<Line> = help_text.into_iter().skip(state.scroll_offset).collect();

    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, inner);
}
