use super::commands::SLASH_COMMANDS;

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
