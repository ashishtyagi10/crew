use std::path::Path;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// State for the command line input at the bottom of the screen.
pub struct CommandLineState {
    /// The current input text.
    pub input: String,
    /// Cursor position within the input (byte offset).
    pub cursor_pos: usize,
    /// Whether the command line is currently accepting input.
    pub visible: bool,
    /// Command history (oldest first).
    pub history: Vec<String>,
    /// Current position in history when browsing (None = not browsing).
    history_index: Option<usize>,
    /// Saved input before history browsing started.
    saved_input: String,
    /// LLM typeahead suggestion (ghost text shown after cursor).
    pub suggestion: Option<String>,
    /// Snapshot of input when suggestion was requested (to discard stale ones).
    pub suggestion_for: String,
    /// Whether a suggestion request is in-flight.
    pub suggestion_pending: bool,
    /// Tick counter for debounce (request after N ticks of no typing).
    pub last_typed_tick: u64,
}

impl CommandLineState {
    /// Create a new empty command line state.
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            visible: true,
            history: Vec::new(),
            history_index: None,
            saved_input: String::new(),
            suggestion: None,
            suggestion_for: String::new(),
            suggestion_pending: false,
            last_typed_tick: 0,
        }
    }

    /// Insert a character at the current cursor position.
    pub fn input_char(&mut self, ch: char) {
        self.input.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
        self.invalidate_suggestion();
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
            self.invalidate_suggestion();
        }
    }

    /// Clear all input text.
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.suggestion = None;
        self.suggestion_pending = false;
    }

    /// Accept the current suggestion (Tab key).
    pub fn accept_suggestion(&mut self) -> bool {
        if let Some(suggestion) = self.suggestion.take() {
            self.input.push_str(&suggestion);
            self.cursor_pos = self.input.len();
            self.suggestion_pending = false;
            true
        } else {
            false
        }
    }

    /// Mark suggestion as stale after typing.
    fn invalidate_suggestion(&mut self) {
        self.suggestion = None;
        self.suggestion_pending = false;
        self.last_typed_tick = 0; // will be set by tick()
    }

    /// Take the current input, clearing the state, and return it.
    pub fn take_input(&mut self) -> String {
        let input = std::mem::take(&mut self.input);
        self.cursor_pos = 0;
        self.history_index = None;
        input
    }

    /// Execute the current input as a shell command and return the output.
    pub fn execute(&mut self) -> Option<String> {
        let input = self.take_input();
        if input.is_empty() {
            return None;
        }

        self.history.push(input.clone());

        let output = if cfg!(windows) {
            std::process::Command::new("cmd")
                .args(["/C", &input])
                .output()
        } else {
            std::process::Command::new("sh")
                .args(["-c", &input])
                .output()
        };

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let result = if stderr.is_empty() {
                    stdout
                } else if stdout.is_empty() {
                    stderr
                } else {
                    format!("{}\n{}", stdout, stderr)
                };
                Some(result.trim().to_string())
            }
            Err(e) => Some(format!("Error: {}", e)),
        }
    }

    /// Navigate to the previous command in history.
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                self.saved_input = self.input.clone();
                let idx = self.history.len() - 1;
                self.history_index = Some(idx);
                self.input = self.history[idx].clone();
                self.cursor_pos = self.input.len();
            }
            Some(idx) if idx > 0 => {
                let new_idx = idx - 1;
                self.history_index = Some(new_idx);
                self.input = self.history[new_idx].clone();
                self.cursor_pos = self.input.len();
            }
            _ => {}
        }
    }

    /// Navigate to the next command in history (or back to the saved input).
    pub fn history_down(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.history.len() {
                let new_idx = idx + 1;
                self.history_index = Some(new_idx);
                self.input = self.history[new_idx].clone();
                self.cursor_pos = self.input.len();
            } else {
                self.history_index = None;
                self.input = std::mem::take(&mut self.saved_input);
                self.cursor_pos = self.input.len();
            }
        }
    }
}

impl Default for CommandLineState {
    fn default() -> Self {
        Self::new()
    }
}

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
    // Truncate dir if too long
    let max_dir_len = (area.width as usize).saturating_sub(20);
    let dir_display = if dir_str.len() > max_dir_len {
        format!(
            "...{}",
            &dir_str[dir_str.len().saturating_sub(max_dir_len)..]
        )
    } else {
        dir_str.to_string()
    };

    let border_color = if has_input {
        Color::Cyan
    } else {
        Color::Indexed(240)
    };

    let block = Block::default()
        .borders(Borders::ALL)
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
                " Type command or ask in English ",
                Style::default().fg(Color::Indexed(244)).bg(Color::Black),
            ),
            Span::styled(
                format!(" {} ", mode_label),
                Style::default()
                    .fg(Color::Black)
                    .bg(mode_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default().bg(Color::Black)),
        ]))
        .border_style(Style::default().fg(border_color).bg(Color::Black))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render the input line inside the box
    let prompt_style = Style::default()
        .fg(if has_input {
            Color::Cyan
        } else {
            Color::Indexed(244)
        })
        .bg(Color::Black);
    let input_style = Style::default().fg(Color::White).bg(if has_input {
        Color::Indexed(235)
    } else {
        Color::Black
    });

    let prompt = "> ";
    let input_width = inner.width.saturating_sub(2) as usize; // 2 for "> "

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

    // Add Tab hint if suggestion is available
    if state.suggestion.is_some() {
        // Show on the title_bottom or as a subtle indicator
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
