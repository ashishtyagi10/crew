//! Slash-command dispatch: maps a `/command` (and its `<arg>` forms) typed in
//! the input bar to the matching `CrewApp` action. Kept in sync with the palette
//! list in `suggest::COMMANDS`.
use crate::app::CrewApp;

impl CrewApp {
    /// Run a `/command` typed in the input bar. Returns `true` if the app should exit.
    pub(crate) fn run_slash_command(&mut self, cmd: &str) -> bool {
        match cmd {
            "exit" => return true,
            "keys" => self.help_open = true,
            "about" => self.show_about(),
            "far" => self.spawn_far_pane(),
            "crew" => self.spawn_crew_pane(),
            "settings" => self.spawn_settings_pane(),
            "shell" => self.spawn_new_pane(),
            "update" => self.spawn_labeled_terminal(
                "sh",
                &["-c".to_string(), "git pull; exec sh".to_string()],
                "update".to_string(),
            ),
            "clear" => self.clear_focused_scrollback(),
            "clearall" => self.clear_all_scrollback(),
            "only" => self.close_other_panes(),
            "closeall" => self.close_all_panes(),
            "copy" => self.copy_scrollback(),
            "dump" => self.dump_focused_pane(""),
            "run" => self.run_in_pane(""),   // show usage hint
            "edit" => self.edit_in_pane(""), // show usage hint
            "open" => self.open_target(""),  // open the last URL on screen
            "pwd" => self.copy_cwd(),
            "font" => self.set_font_cmd(""),
            "reload" => self.reload_config(),
            "broadcast" => self.toggle_broadcast(),
            "zoom" => self.toggle_zoom(),
            "sidebar" => self.toggle_sidebar(),
            "name" => self.name_focused_pane(""), // clear the pane's name
            other => {
                if let Some(term) = other.strip_prefix("find ") {
                    self.find_in_terminal(term.trim());
                } else if let Some(n) = other.strip_prefix("name ") {
                    self.name_focused_pane(n.trim());
                } else if let Some(t) = other.strip_prefix("open ") {
                    self.open_target(t);
                } else if let Some(c) = other.strip_prefix("run ") {
                    self.run_in_pane(c);
                } else if let Some(f) = other.strip_prefix("edit ") {
                    self.edit_in_pane(f);
                } else if let Some(f) = other.strip_prefix("dump ") {
                    self.dump_focused_pane(f);
                } else if let Some(n) = other.strip_prefix("font ") {
                    self.set_font_cmd(n);
                }
            }
        }
        false
    }
}
