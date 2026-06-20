//! Slash commands for managing the agent grid: list tiles, focus a tile by
//! number, and rename the focused tile. All are command-driven so they never
//! steal keys from the focused agent.

use super::super::App;

impl App {
    /// Dispatch agent-grid slash commands. Returns `true` if `cmd` matched.
    pub(super) fn slash_agents(&mut self, cmd: &str, args: &str) -> bool {
        match cmd {
            "/agents" | "/ls" => self.list_agents(),
            "/focus" | "/f" => self.focus_agent_by_arg(args),
            "/title" => self.rename_focused_agent(args),
            _ => return false,
        }
        true
    }

    /// Agent tile ids in display order: full tiles first (most-recently-active),
    /// then minimized ones. This is the numbering `/agents` and `/focus` use.
    fn agent_order(&self) -> Vec<usize> {
        self.grid
            .full()
            .iter()
            .chain(self.grid.minimized().iter())
            .copied()
            .collect()
    }

    /// `/agents` — list every running tile with its number, title, state and cwd.
    fn list_agents(&mut self) {
        let order = self.agent_order();
        if order.is_empty() {
            self.feedback.info("No agents running".to_string());
            return;
        }
        let minimized = self.grid.minimized().to_vec();
        let lines: Vec<String> = order
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let n = i + 1;
                let (title, cwd, alive, attention) = match self.terminal_by_id(*id) {
                    Some(t) => (
                        t.title.clone(),
                        t.cwd.display().to_string(),
                        t.alive,
                        t.has_attention,
                    ),
                    None => (String::from("?"), String::new(), false, false),
                };
                let mut flags = Vec::new();
                if self.focused_terminal == Some(*id) {
                    flags.push("focused");
                }
                if minimized.contains(id) {
                    flags.push("minimized");
                }
                if !alive {
                    flags.push("exited");
                }
                if attention {
                    flags.push("●");
                }
                let suffix = if flags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", flags.join(", "))
                };
                format!("  {}. {}{} — {}", n, title, suffix, cwd)
            })
            .collect();
        self.feedback.show_output("Agents", lines.join("\n"));
    }

    /// `/focus N` — focus the tile numbered `N` (1-based, as shown by `/agents`).
    fn focus_agent_by_arg(&mut self, args: &str) {
        let order = self.agent_order();
        if order.is_empty() {
            self.feedback.info("No agents to focus".to_string());
            return;
        }
        let n: usize = match args.trim().parse() {
            Ok(n) if n >= 1 && n <= order.len() => n,
            _ => {
                self.feedback
                    .error(format!("Usage: /focus 1..{}", order.len()));
                return;
            }
        };
        let id = order[n - 1];
        self.focused_terminal = Some(id);
        // Pull a minimized tile up into the full grid; leave full tiles in place.
        if self.grid.minimized().contains(&id) {
            self.grid.touch(id);
        }
        let title = match self.terminal_by_id_mut(id) {
            Some(t) => {
                t.has_attention = false;
                t.title.clone()
            }
            None => return,
        };
        self.feedback.info(format!("Focused {}. {}", n, title));
    }

    /// `/title <text>` — rename the focused agent tile.
    fn rename_focused_agent(&mut self, args: &str) {
        let title = args.trim();
        if title.is_empty() {
            self.feedback
                .error("Usage: /title <new tile name>".to_string());
            return;
        }
        let Some(id) = self.focused_terminal else {
            self.feedback
                .error("No focused agent to rename".to_string());
            return;
        };
        if let Some(t) = self.terminal_by_id_mut(id) {
            t.title = title.to_string();
        }
        self.feedback.info(format!("Renamed tile to {}", title));
    }
}
