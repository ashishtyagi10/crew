//! Slash-command dispatch: maps a `/command` (and its `<arg>` forms) typed in
//! the input bar to the matching `CrewApp` action. Kept in sync with the palette
//! list in `suggest::COMMANDS`.
//!
//! The `<arg>` forms live in [`crate::dispatchargs`]; the appearance knobs in
//! [`crate::dispatchlook`], [`crate::dispatchtype`] and
//! [`crate::dispatchspace`]; `/todo` and `/notify` in
//! [`crate::dispatchtodo`]. Split for the line cap, along the boundary the
//! commands already had.
use crate::app::CrewApp;

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;

impl CrewApp {
    /// Run a `/command` typed in the input bar. Returns `true` if the app should exit.
    pub(crate) fn run_slash_command(&mut self, cmd: &str) -> bool {
        self.note_command_run(cmd);
        // A confirmation you have moved on from is not still armed. `/only`
        // and `/close all` re-arm themselves below; anything else in between
        // means you went and did something else, and a `/closeall` typed
        // after that should ask again rather than fire on the first press.
        if self.pending.armed() && !matches!(cmd, "only" | "closeall") {
            self.pending.clear();
        }
        match cmd {
            "exit" => return true,
            "keys" => self.help_open = true,
            // A long build scrolls its own failure off the screen; this walks
            // back to it. Repeating steps to the one before.
            "errors" => self.find_error_in_terminal(),
            "errorsall" => self.find_errors_everywhere(),
            // The last command's output, on its own, in a pane you can read.
            "out" => self.open_last_output(""),
            // …and the list the number in `/out <n>` counts through.
            "blocks" => self.open_blocks(),
            "tools" => self.open_tools(""),
            "watching" => self.open_watching(""),
            "pin" => self.toggle_pin(),
            "marks" => self.marks_command(""),
            "invisibles" => self.invisibles_command(""),
            "far" => self.spawn_far_pane(),
            "goal" => self.spawn_goal_pane(""), // show usage hint
            "model" => self.set_model_cmd(""),  // show usage hint
            "batch" => self.spawn_batch_pane(""), // show usage hint
            "md" | "view" => self.open_view(""), // show usage hint
            "doc" => self.set_status("usage: /doc <path>"),
            // Who last touched each line of the file in the viewer.
            "blame" => self.blame_command(),
            "smith" | "crew" => self.spawn_crew_pane(), // /crew kept as an alias
            "settings" => self.spawn_settings_pane(),
            "todo" => self.spawn_todo_pane(),
            "usage" => self.spawn_usage_pane(),
            "disk" => self.spawn_disk_pane(None),
            "dash" => self.spawn_dash_pane(),
            "shell" => self.spawn_new_pane(),
            // Reopen last session's shells (their cwds snapshot on quit).
            "restore" => self.restore_session(),
            // Undo the last close, this session — `/restore` is the same idea
            // across a quit. See `reopen`.
            "reopen" => self.reopen_pane(),
            // Self-update in the background: progress shows in the left-nav UPDATE
            // card; once the install lands, Crew restarts itself into the new
            // build (an already-parked install restarts immediately).
            "update" => return self.start_update(),
            // Absorbed into /update (which now restarts after installing). A
            // bare status beats the fuzzy matcher here, which would otherwise
            // suggest /restore — a different action entirely.
            "restart" => self.set_status("/restart merged into /update — it installs and restarts"),
            "clear" => self.clear_focused_scrollback(),
            "clearall" => self.clear_all_scrollback(),
            "clearlog" => self.clear_log(),
            "only" => self.close_other_panes(),
            "closeall" => self.close_all_panes(),
            "pwd" => self.copy_cwd(),
            "about" => self.spawn_about_pane(),
            "log" => self.open_log(),
            "copy" => self.copy_scrollback(),
            "copy out" => self.copy_last_output(),
            "dump" => self.dump_focused_pane(""),
            "diff" => self.diff_in_pane(),
            "run" => self.run_in_pane(""), // show usage hint
            "font" => self.set_font_cmd(""),
            "theme" => self.set_theme_cmd(""),
            "crt" => self.crt_command(""),
            "weight" => self.weight_command(""),
            "smooth" => self.smooth_command(""),
            "gamma" => self.gamma_command(""),
            "motion" => self.motion_command(""),
            "density" => self.density_command(""),
            "leading" => self.leading_command(""),
            "contrast" => self.contrast_command(""),
            "shapes" => self.shapes_command(""),
            "gradient" => self.gradient_command(""),
            "opacity" => self.opacity_command(""),
            "grain" => self.grain_command(""),
            "notify" => self.notify_command(""),
            "broadcast" => self.toggle_broadcast(),
            "zoom" => self.toggle_zoom(),
            "focus" => self.toggle_focus_mode(),
            "sidebar" => self.toggle_sidebar(),
            "name" => self.name_focused_pane(""), // clear the pane's name
            "findall" => self.find_all(""),       // show usage hint
            // Everything that carries an argument. The chain moved WHOLE
            // rather than by subject: its order is load-bearing (`find ` must
            // be tried after `findall `, or `/findall x` routes to `/find`),
            // and an order spread across files is one nobody can read.
            other => self.run_slash_with_arg(other),
        }
        false
    }

    /// Remember that this command was run, so the palette can rank what the
    /// user actually uses above the order `cmddefs` happens to declare.
    ///
    /// Only commands that EXIST are recorded — the argument here is the whole
    /// typed line (`"theme dark"`), so it is matched back to a table entry by
    /// its first word. A typo is not a habit, and recording one would put a
    /// command that does not exist at the top of the list.
    fn note_command_run(&mut self, cmd: &str) {
        let head = cmd.split_whitespace().next().unwrap_or(cmd);
        let name = format!("/{head}");
        if !crate::cmddefs::commands().any(|c| c.name == name) {
            return;
        }
        let list = crate::cmdrecents::record(&name);
        if self.config.command_recents != list {
            self.config.command_recents = list;
            self.config.save();
        }
    }
}
