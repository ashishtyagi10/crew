use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// A single slash command entry for the suggestion list.
#[derive(Debug, Clone)]
pub struct SlashCommand {
    /// Primary command (e.g. "/cd").
    pub command: &'static str,
    /// Short description shown next to the command.
    pub description: &'static str,
}

/// All available slash commands with their descriptions.
pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        command: "/actions",
        description: "Quick actions for selected file",
    },
    SlashCommand {
        command: "/ai",
        description: "Open AI assistant bar",
    },
    SlashCommand {
        command: "/ai-tools",
        description: "AI coding tools panel",
    },
    SlashCommand {
        command: "/back",
        description: "Navigate to previous directory",
    },
    SlashCommand {
        command: "/bookmark",
        description: "Show / add bookmarks",
    },
    SlashCommand {
        command: "/cd",
        description: "Change directory",
    },
    SlashCommand {
        command: "/checksum",
        description: "Show SHA-256 checksums",
    },
    SlashCommand {
        command: "/chmod",
        description: "Change file permissions",
    },
    SlashCommand {
        command: "/claude",
        description: "Launch Claude Code",
    },
    SlashCommand {
        command: "/codex",
        description: "Launch OpenAI Codex",
    },
    SlashCommand {
        command: "/compare",
        description: "Compare left & right directories",
    },
    SlashCommand {
        command: "/compress",
        description: "Compress selected files",
    },
    SlashCommand {
        command: "/copilot",
        description: "Launch GitHub Copilot",
    },
    SlashCommand {
        command: "/deselect",
        description: "Deselect files by mask",
    },
    SlashCommand {
        command: "/duplicates",
        description: "Find duplicate files",
    },
    SlashCommand {
        command: "/exit",
        description: "Exit Farx",
    },
    SlashCommand {
        command: "/extract",
        description: "Extract archive",
    },
    SlashCommand {
        command: "/filter",
        description: "Filter file listing by pattern",
    },
    SlashCommand {
        command: "/find-file",
        description: "Fuzzy file finder",
    },
    SlashCommand {
        command: "/forward",
        description: "Navigate forward in history",
    },
    SlashCommand {
        command: "/grep",
        description: "Search inside files by content",
    },
    SlashCommand {
        command: "/gemini",
        description: "Launch Google Gemini",
    },
    SlashCommand {
        command: "/goto",
        description: "Navigate to a directory",
    },
    SlashCommand {
        command: "/help",
        description: "Show help screen",
    },
    SlashCommand {
        command: "/hidden",
        description: "Toggle hidden files",
    },
    SlashCommand {
        command: "/info",
        description: "Toggle file info panel",
    },
    SlashCommand {
        command: "/invert",
        description: "Invert file selection",
    },
    SlashCommand {
        command: "/menu",
        description: "Open menu bar",
    },
    SlashCommand {
        command: "/open",
        description: "Open with system application",
    },
    SlashCommand {
        command: "/opencode",
        description: "Launch OpenCode AI agent",
    },
    SlashCommand {
        command: "/plugin",
        description: "List or run plugins",
    },
    SlashCommand {
        command: "/recent",
        description: "Recent directories",
    },
    SlashCommand {
        command: "/refresh",
        description: "Refresh both panels",
    },
    SlashCommand {
        command: "/rename-batch",
        description: "Batch rename files",
    },
    SlashCommand {
        command: "/search",
        description: "Search in files",
    },
    SlashCommand {
        command: "/shell",
        description: "Open embedded terminal shell",
    },
    SlashCommand {
        command: "/select",
        description: "Select files by mask",
    },
    SlashCommand {
        command: "/size",
        description: "Calculate directory size",
    },
    SlashCommand {
        command: "/sort",
        description: "Sort by name|ext|size|date",
    },
    SlashCommand {
        command: "/ssh",
        description: "Browse remote via SSH",
    },
    SlashCommand {
        command: "/stats",
        description: "Show file statistics",
    },
    SlashCommand {
        command: "/swap",
        description: "Swap left & right panels",
    },
    SlashCommand {
        command: "/symlink",
        description: "Create symbolic link",
    },
    SlashCommand {
        command: "/terminal",
        description: "Open terminal here",
    },
    SlashCommand {
        command: "/touch",
        description: "Create empty file",
    },
    SlashCommand {
        command: "/treemap",
        description: "Show disk usage treemap",
    },
    SlashCommand {
        command: "/undo",
        description: "Undo last file operation",
    },
    SlashCommand {
        command: "/yank",
        description: "Copy path to clipboard",
    },
];

/// State for the slash command suggestion popup.
pub struct SlashSuggestionsState {
    /// Filtered list of matching command indices into SLASH_COMMANDS.
    pub matches: Vec<usize>,
    /// Current cursor position within matches.
    pub cursor: usize,
}

impl SlashSuggestionsState {
    /// Build suggestions filtered by the current input prefix.
    /// Input should include the leading `/` (e.g. "/cd", "/so").
    pub fn new(input: &str) -> Self {
        let query = input.to_lowercase();
        let matches: Vec<usize> = SLASH_COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, cmd)| cmd.command.starts_with(&query))
            .map(|(i, _)| i)
            .collect();
        Self { matches, cursor: 0 }
    }

    /// Move cursor up.
    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down.
    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.matches.len() {
            self.cursor += 1;
        }
    }

    /// Get the selected command string, if any.
    pub fn selected_command(&self) -> Option<&'static str> {
        self.matches
            .get(self.cursor)
            .map(|&i| SLASH_COMMANDS[i].command)
    }
}

/// Render the slash suggestion popup just above the command line area.
///
/// `cmd_area` is the Rect of the command line box — the popup floats above it.
pub fn render_slash_suggestions(frame: &mut Frame, state: &SlashSuggestionsState, cmd_area: Rect) {
    if state.matches.is_empty() {
        return;
    }

    let max_visible = 12u16;
    let item_count = state.matches.len() as u16;
    // +2 for top/bottom border
    let popup_height = item_count.min(max_visible) + 2;
    let popup_width = 48u16.min(cmd_area.width);

    // Position above the command line
    let y = cmd_area.y.saturating_sub(popup_height);
    let popup_area = Rect::new(cmd_area.x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Commands ")
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Scroll the view so cursor is always visible
    let visible_rows = inner.height as usize;
    let scroll_offset = if state.cursor >= visible_rows {
        state.cursor - visible_rows + 1
    } else {
        0
    };

    for (row, &cmd_idx) in state
        .matches
        .iter()
        .skip(scroll_offset)
        .take(visible_rows)
        .enumerate()
    {
        let cmd = &SLASH_COMMANDS[cmd_idx];
        let is_selected = scroll_offset + row == state.cursor;

        let (cmd_style, desc_style) = if is_selected {
            (
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Indexed(24))
                    .add_modifier(Modifier::BOLD),
                Style::default()
                    .fg(Color::Indexed(250))
                    .bg(Color::Indexed(24)),
            )
        } else {
            (
                Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
                Style::default()
                    .fg(Color::Indexed(244))
                    .bg(Color::Indexed(236)),
            )
        };

        let available = inner.width as usize;
        let cmd_text = cmd.command;
        let desc_text = cmd.description;
        // Pad between command and description
        let gap = available
            .saturating_sub(cmd_text.len())
            .saturating_sub(desc_text.len())
            .saturating_sub(2); // 1 space prefix + 1 min gap

        let line = Line::from(vec![
            Span::styled(" ", cmd_style),
            Span::styled(cmd_text, cmd_style),
            Span::styled(" ".repeat(gap.max(1)), desc_style),
            Span::styled(desc_text, desc_style),
        ]);

        let row_area = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        // Fill background for selected row
        if is_selected {
            let bg_fill = " ".repeat(inner.width as usize);
            frame.render_widget(
                Paragraph::new(Span::styled(
                    bg_fill,
                    Style::default().bg(Color::Indexed(24)),
                )),
                row_area,
            );
        }
        frame.render_widget(Paragraph::new(line), row_area);
    }
}
