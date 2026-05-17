//! History navigation and shell command execution.

use super::state::CommandLineState;

impl CommandLineState {
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
