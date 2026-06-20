//! Slash command routing. The top-level `handle_slash_command` tries each
//! category dispatcher in turn; the first one that recognises the command
//! returns `true` and execution stops.

mod agents;
mod ai;
mod core;
mod files;
mod plugin;

use super::App;

impl App {
    /// Handle a slash command. Returns `true` if the command was recognised.
    pub(super) fn handle_slash_command(&mut self, input: &str) -> bool {
        let trimmed = input.trim();
        let (cmd, args) = match trimmed.split_once(char::is_whitespace) {
            Some((c, a)) => (c, a.trim()),
            None => (trimmed, ""),
        };

        self.slash_core(cmd, args)
            || self.slash_agents(cmd, args)
            || self.slash_ai(cmd, args)
            || self.slash_files(cmd, args)
            || self.slash_plugin_or_unknown(cmd, args)
    }
}
