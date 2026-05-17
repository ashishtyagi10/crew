//! Command-line execution: `cd` parsing, slash-command routing, shell vs
//! AI classification, and slash suggestion popup updates.

use std::path::PathBuf;

use farx_core::PanelSide;

use crate::components::ai_bar::AiBarState;
use crate::components::slash_suggestions::SlashSuggestionsState;

use super::shell_commands::looks_like_shell_command;
use super::App;

impl App {
    /// Execute whatever's in the command line: slash command, `cd`, shell, or
    /// natural language (routed to the AI bar).
    pub(super) fn smart_execute_command(&mut self) {
        let input = self.command_line.take_input();
        if input.is_empty() {
            return;
        }

        self.command_line.history.push(input.clone());

        if input.starts_with('/') && self.handle_slash_command(&input) {
            return;
        }

        if let Some(cd_arg) = Self::parse_cd_command(&input) {
            let base = self.active_panel_ref().current_dir.clone();
            let path = if cd_arg.is_empty() {
                dirs::home_dir().unwrap_or(base)
            } else if cd_arg == "-" {
                let history = match self.active_panel {
                    PanelSide::Left => &self.left_tree.history_back,
                    PanelSide::Right => &self.right_tree.history_back,
                };
                if let Some(prev) = history.last() {
                    prev.clone()
                } else {
                    self.feedback.error("No previous directory".to_string());
                    return;
                }
            } else if cd_arg.starts_with('~') {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join(cd_arg.trim_start_matches("~/").trim_start_matches('~'))
            } else if cd_arg.starts_with('/') {
                PathBuf::from(&cd_arg)
            } else {
                base.join(&cd_arg)
            };
            match path.canonicalize() {
                Ok(resolved) if resolved.is_dir() => {
                    self.navigate_to(resolved);
                }
                Ok(resolved) => {
                    self.feedback
                        .error(format!("Not a directory: {}", resolved.display()));
                }
                Err(_) => {
                    self.feedback
                        .error(format!("No such directory: {}", cd_arg));
                }
            }
            return;
        }

        if looks_like_shell_command(&input) {
            let output = if cfg!(windows) {
                std::process::Command::new("cmd")
                    .args(["/C", &input])
                    .current_dir(&self.active_panel_ref().current_dir)
                    .output()
            } else {
                std::process::Command::new("sh")
                    .args(["-c", &input])
                    .current_dir(&self.active_panel_ref().current_dir)
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
                    let result = result.trim().to_string();
                    if result.lines().count() <= 1 {
                        if !result.is_empty() {
                            self.feedback.info(result);
                        }
                    } else {
                        self.feedback.show_output("Output", result);
                    }
                }
                Err(e) => {
                    self.feedback.error(format!("Command: {}", e));
                }
            }
            self.left_tree.rebuild();
            self.right_tree.rebuild();
        } else {
            self.ai_bar = Some(AiBarState::new());
            if let Some(ref mut ai_bar) = self.ai_bar {
                ai_bar.input = input.clone();
                ai_bar.cursor_pos = input.len();
                ai_bar.thinking = true;
            }
            self.submit_ai_query(input);
        }
    }

    /// Parse a `cd` command, returning the argument (possibly empty for bare
    /// `cd`). Returns `None` if the input is not a `cd` command.
    pub(super) fn parse_cd_command(input: &str) -> Option<String> {
        let trimmed = input.trim();
        if trimmed == "cd" {
            return Some(String::new());
        }
        if let Some(rest) = trimmed.strip_prefix("cd ") {
            return Some(rest.trim().to_string());
        }
        if let Some(rest) = trimmed.strip_prefix("cd\t") {
            return Some(rest.trim().to_string());
        }
        None
    }

    /// Refresh the slash command suggestion popup based on current input.
    pub(super) fn update_slash_suggestions(&mut self) {
        let input = &self.command_line.input;
        if input.starts_with('/') && !input.contains(' ') {
            let state = SlashSuggestionsState::new(input);
            if state.matches.is_empty() {
                self.slash_suggestions = None;
            } else {
                self.slash_suggestions = Some(state);
            }
        } else {
            self.slash_suggestions = None;
        }
    }
}
