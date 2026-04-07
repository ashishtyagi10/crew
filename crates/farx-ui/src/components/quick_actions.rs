use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct QuickAction {
    pub label: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuickActionResult {
    None,
    Close,
    Execute(String),
}

pub struct QuickActionsState {
    pub active: bool,
    pub actions: Vec<QuickAction>,
    pub cursor: usize,
    pub file_name: String,
}

impl QuickActionsState {
    pub fn new(file_name: String, extension: Option<&str>, is_dir: bool) -> Self {
        let mut actions = Vec::new();

        // Universal actions
        actions.push(QuickAction {
            label: "Open with system default".to_string(),
            command: "__open__".to_string(),
        });

        if is_dir {
            actions.push(QuickAction {
                label: "Open terminal here".to_string(),
                command: "__terminal__".to_string(),
            });
            actions.push(QuickAction {
                label: "Count files recursively".to_string(),
                command: "find . -type f | wc -l".to_string(),
            });
        } else {
            actions.push(QuickAction {
                label: "View in editor".to_string(),
                command: "__edit__".to_string(),
            });
            actions.push(QuickAction {
                label: "View file".to_string(),
                command: "__view__".to_string(),
            });
            actions.push(QuickAction {
                label: "Copy path to clipboard".to_string(),
                command: "__clipboard__".to_string(),
            });

            // Extension-specific actions
            match extension {
                Some("rs") => {
                    actions.push(QuickAction {
                        label: "Cargo check".to_string(),
                        command: "cargo check".to_string(),
                    });
                    actions.push(QuickAction {
                        label: "Cargo test".to_string(),
                        command: "cargo test".to_string(),
                    });
                    actions.push(QuickAction {
                        label: "Cargo run".to_string(),
                        command: "cargo run".to_string(),
                    });
                }
                Some("py") => {
                    actions.push(QuickAction {
                        label: "Run with Python".to_string(),
                        command: format!("python3 {}", file_name),
                    });
                    actions.push(QuickAction {
                        label: "Lint (ruff)".to_string(),
                        command: format!("ruff check {}", file_name),
                    });
                }
                Some("js") | Some("ts") | Some("jsx") | Some("tsx") => {
                    actions.push(QuickAction {
                        label: "Run with Node".to_string(),
                        command: format!("node {}", file_name),
                    });
                    actions.push(QuickAction {
                        label: "Lint (eslint)".to_string(),
                        command: format!("npx eslint {}", file_name),
                    });
                }
                Some("sh") | Some("bash") | Some("zsh") => {
                    actions.push(QuickAction {
                        label: "Run script".to_string(),
                        command: format!("sh {}", file_name),
                    });
                    actions.push(QuickAction {
                        label: "Make executable".to_string(),
                        command: format!("chmod +x {}", file_name),
                    });
                }
                Some("go") => {
                    actions.push(QuickAction {
                        label: "Go run".to_string(),
                        command: format!("go run {}", file_name),
                    });
                    actions.push(QuickAction {
                        label: "Go test".to_string(),
                        command: "go test ./...".to_string(),
                    });
                }
                Some("json") => {
                    actions.push(QuickAction {
                        label: "Pretty print (jq)".to_string(),
                        command: format!("jq . {}", file_name),
                    });
                }
                Some("md") | Some("markdown") => {
                    actions.push(QuickAction {
                        label: "Word count".to_string(),
                        command: format!("wc -w {}", file_name),
                    });
                }
                Some("zip") | Some("tar") | Some("gz") | Some("tgz") => {
                    actions.push(QuickAction {
                        label: "Extract archive".to_string(),
                        command: "__extract__".to_string(),
                    });
                    actions.push(QuickAction {
                        label: "List contents".to_string(),
                        command: "__view_archive__".to_string(),
                    });
                }
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg")
                | Some("webp") => {
                    actions.push(QuickAction {
                        label: "Get image dimensions".to_string(),
                        command: format!("file {}", file_name),
                    });
                }
                _ => {}
            }

            // Generic file actions
            actions.push(QuickAction {
                label: "File info (stat)".to_string(),
                command: format!("stat {}", file_name),
            });
            actions.push(QuickAction {
                label: "Checksum (SHA-256)".to_string(),
                command: format!("shasum -a 256 {}", file_name),
            });
        }

        Self {
            active: true,
            actions,
            cursor: 0,
            file_name,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> QuickActionResult {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                QuickActionResult::Close
            }
            KeyCode::Enter => {
                if let Some(action) = self.actions.get(self.cursor) {
                    self.active = false;
                    QuickActionResult::Execute(action.command.clone())
                } else {
                    QuickActionResult::None
                }
            }
            KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                QuickActionResult::None
            }
            KeyCode::Down => {
                if self.cursor + 1 < self.actions.len() {
                    self.cursor += 1;
                }
                QuickActionResult::None
            }
            _ => QuickActionResult::None,
        }
    }
}

pub fn render_quick_actions(frame: &mut Frame, state: &QuickActionsState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 50u16.min(area.width.saturating_sub(4));
    let dialog_height = (state.actions.len() as u16 + 4).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Actions: {} ", state.file_name))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let visible = (inner.height.saturating_sub(1)) as usize;
    for (i, action) in state.actions.iter().take(visible).enumerate() {
        let is_selected = i == state.cursor;
        let style = if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(Color::Indexed(24))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
        };

        let display = format!(" {} ", action.label);
        let truncated: String = display.chars().take(inner.width as usize).collect();
        frame.render_widget(
            Paragraph::new(Span::styled(truncated, style)),
            Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
        );
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " Enter=Run  Esc=Close",
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
