//! Slash commands that launch the AI bar, the AI tools panel, or an
//! embedded terminal running a specific CLI assistant.

use std::path::PathBuf;

use crate::components::ai_bar::AiBarState;
use crate::components::ai_panel::AiPanelState;

use super::super::App;

impl App {
    /// Dispatch AI/shell slash commands. Returns `true` if `cmd` matched.
    ///
    /// Agent/shell commands accept an optional directory argument, e.g.
    /// `/claude ~/project` or `/shell ./src`, and launch there. With no
    /// argument they use the current directory (changeable via `/cd`).
    pub(super) fn slash_ai(&mut self, cmd: &str, args: &str) -> bool {
        match cmd {
            "/ai" => self.ai_bar = Some(AiBarState::new()),
            "/ai-tools" | "/ait" => self.ai_panel = Some(AiPanelState::new()),
            "/claude" => self.spawn_agent("claude", &[], args),
            "/codex" => self.spawn_agent("codex", &[], args),
            "/copilot" => self.spawn_agent("gh", &["copilot"], args),
            "/gemini" => self.spawn_agent("gemini", &[], args),
            "/opencode" => self.spawn_agent("opencode", &[], args),
            "/shell" | "/sh" | "/bash" | "/zsh" => {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                self.spawn_agent(&shell, &[], args);
            }
            // Close the focused panel — falls back to the most-recently-active
            // one so it still works right after Alt+Enter cleared the focus.
            "/close" | "/x" => {
                match self
                    .focused_terminal
                    .or_else(|| self.grid.full().first().copied())
                {
                    Some(id) => self.close_terminal(id),
                    None => self.feedback.error("No panel to close".to_string()),
                }
            }
            "/closeall" => {
                let ids: Vec<usize> = self
                    .grid
                    .full()
                    .iter()
                    .chain(self.grid.minimized().iter())
                    .copied()
                    .collect();
                let n = ids.len();
                for id in ids {
                    self.close_terminal(id);
                }
                self.feedback.info(format!("Closed {} panel(s)", n));
            }
            _ => return false,
        }
        true
    }

    /// Spawn an agent/shell, honoring an optional directory argument.
    fn spawn_agent(&mut self, program: &str, cli_args: &[&str], dir_arg: &str) {
        let dir = if dir_arg.is_empty() {
            self.active_tree_ref().root.clone()
        } else {
            match self.resolve_agent_dir(dir_arg) {
                Some(d) => d,
                None => {
                    self.feedback.error(format!("Not a directory: {}", dir_arg));
                    return;
                }
            }
        };
        self.spawn_embedded_terminal_in(program, cli_args, dir);
    }

    /// Resolve a directory argument (absolute, `~`, or relative to the current
    /// directory) to an existing directory, or `None` if it isn't one.
    fn resolve_agent_dir(&self, arg: &str) -> Option<PathBuf> {
        let path = if arg == "~" {
            dirs::home_dir()?
        } else if let Some(rest) = arg.strip_prefix("~/") {
            dirs::home_dir()?.join(rest)
        } else if arg.starts_with('/') {
            PathBuf::from(arg)
        } else {
            self.active_tree_ref().root.join(arg)
        };
        path.is_dir().then_some(path)
    }
}
